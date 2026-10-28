//! SAME processed owner, selected `HiLOD` history inputs and output copies.
//! Prior source provenance remains on the existing prior-output fields. These
//! distinct +2c0/+924 outputs record their own actual Reset/publication origin.
use super::SkeletonProcessedContact;
use crate::{
    physical_player_phys_out::{PhysOutOrigin, PhysOutValue},
    physical_player_prepare_velocity::PhysicalPlayerPrepareVelocitySource,
    skeleton_processed_contact::jump_events::PREPARE,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::{fmt, sync::Arc};

pub(crate) struct SkeletonProcessedPrepareOutput {
    velocity: Option<PhysOutValue<SkeletonRootVector>>,
    count: Option<PhysOutValue<u32>>,
    complete: bool,
}
impl SkeletonProcessedPrepareOutput {
    pub(crate) const fn uninitialized() -> Self {
        Self {
            velocity: None,
            count: None,
            complete: false,
        }
    }
    pub(crate) fn reset_source_fields(&mut self) {
        // Reset full zero +2c0 at39348, then +924 count zero at39528.
        self.velocity = Some(PhysOutValue {
            value: SkeletonRootVector::known([0.0; 4]),
            origin: PhysOutOrigin::Reset,
        });
        self.count = Some(PhysOutValue {
            value: 0,
            origin: PhysOutOrigin::Reset,
        });
        self.complete = false;
    }
    pub(crate) const fn velocity(&self) -> Option<PhysOutValue<SkeletonRootVector>> {
        self.velocity
    }
    pub(crate) const fn count(&self) -> Option<PhysOutValue<u32>> {
        self.count
    }
    pub(crate) const fn complete(&self) -> bool {
        self.complete
    }
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SkeletonProcessedPrepareError {
    ForeignCapture,
    UnknownPrepare,
    MissingVelocity,
    MissingSurfaceVelocity,
    UnknownSpin,
}
impl fmt::Display for SkeletonProcessedPrepareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ForeignCapture => "prepared history capture belongs to another processed source",
            Self::UnknownPrepare => "processed PREPARE flag is unknown",
            Self::MissingVelocity => "processed prior velocity copy is unready",
            Self::MissingSurfaceVelocity => "processed prior surface velocity copy is unready",
            Self::UnknownSpin => "processed Spin is unknown",
        })
    }
}
pub(crate) struct SkeletonProcessedPrepareToken(Arc<()>);
pub(crate) struct SkeletonProcessedPrepareSource<'a> {
    processed: &'a mut SkeletonProcessedContact,
    identity: Arc<()>,
}
impl<'a> SkeletonProcessedPrepareSource<'a> {
    /// Immutable-world/one-processed-owner borrow scope; capture never copies
    /// values in advance. This readiness invalidation is host bookkeeping only.
    pub(crate) fn new(processed: &'a mut SkeletonProcessedContact) -> Self {
        processed.prepare_output.complete = false;
        Self {
            processed,
            identity: Arc::new(()),
        }
    }
    fn validate(
        &self,
        token: &SkeletonProcessedPrepareToken,
    ) -> Result<(), SkeletonProcessedPrepareError> {
        if Arc::ptr_eq(&token.0, &self.identity) {
            Ok(())
        } else {
            Err(SkeletonProcessedPrepareError::ForeignCapture)
        }
    }
}
impl PhysicalPlayerPrepareVelocitySource for SkeletonProcessedPrepareSource<'_> {
    type Processed = SkeletonProcessedPrepareToken;
    type Error = SkeletonProcessedPrepareError;
    fn capture_processed(&mut self) -> Result<Self::Processed, Self::Error> {
        Ok(SkeletonProcessedPrepareToken(Arc::clone(&self.identity)))
    }
    fn prepare(&mut self, processed: &Self::Processed) -> Result<bool, Self::Error> {
        self.validate(processed)?;
        let (flags, known) = self.processed.jump_events().known_flags();
        if known & PREPARE == 0 {
            return Err(Self::Error::UnknownPrepare);
        }
        Ok(flags & PREPARE != 0)
    }
    fn velocity(&mut self, processed: &Self::Processed) -> Result<SkeletonRootVector, Self::Error> {
        self.validate(processed)?;
        // Constructor/Reset values are legitimate native inputs. Origin remains
        // on the original prior record; no synthetic Publication is assigned.
        self.processed
            .prior_output()
            .velocity()
            .map(|value| value.value)
            .ok_or(Self::Error::MissingVelocity)
    }
    fn surface_velocity(
        &mut self,
        processed: &Self::Processed,
    ) -> Result<SkeletonRootVector, Self::Error> {
        self.validate(processed)?;
        self.processed
            .prior_output()
            .surface_velocity()
            .map(|value| value.value)
            .ok_or(Self::Error::MissingSurfaceVelocity)
    }
    fn spin(&mut self, processed: &Self::Processed) -> Result<f32, Self::Error> {
        self.validate(processed)?;
        self.processed
            .jump_events()
            .spin()
            .ok_or(Self::Error::UnknownSpin)
    }
    fn publish_spin_count(
        &mut self,
        processed: &Self::Processed,
        count: u32,
    ) -> Result<(), Self::Error> {
        self.validate(processed)?;
        self.processed.prepare_output.count = Some(PhysOutValue {
            value: count,
            origin: PhysOutOrigin::Publication,
        });
        Ok(())
    }
    fn publish_prepare_velocity(
        &mut self,
        processed: &Self::Processed,
        velocity: SkeletonRootVector,
    ) -> Result<(), Self::Error> {
        self.validate(processed)?;
        self.processed.prepare_output.velocity = Some(PhysOutValue {
            value: velocity,
            origin: PhysOutOrigin::Publication,
        });
        self.processed.prepare_output.complete = true;
        Ok(())
    }
}
impl SkeletonProcessedContact {
    pub(crate) fn prepare_output(&self) -> &SkeletonProcessedPrepareOutput {
        &self.prepare_output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn foreign_capture_refuses_without_publishing_and_partial_count_is_distinct() {
        let mut first = SkeletonProcessedContact::new();
        let mut second = SkeletonProcessedContact::new();
        let mut a = SkeletonProcessedPrepareSource::new(&mut first);
        let mut b = SkeletonProcessedPrepareSource::new(&mut second);
        let own = a.capture_processed().unwrap();
        let foreign = b.capture_processed().unwrap();
        assert_eq!(
            a.prepare(&own),
            Err(SkeletonProcessedPrepareError::UnknownPrepare)
        );
        assert_eq!(
            a.spin(&own),
            Err(SkeletonProcessedPrepareError::UnknownSpin)
        );
        assert_eq!(
            a.publish_spin_count(&foreign, 7),
            Err(SkeletonProcessedPrepareError::ForeignCapture)
        );
        assert!(a.processed.prepare_output().count().is_none());
        a.publish_spin_count(&own, 7).unwrap();
        assert_eq!(
            a.processed.prepare_output().count().unwrap().origin,
            PhysOutOrigin::Publication
        );
        assert!(a.processed.prepare_output().velocity().is_none());
        assert!(!a.processed.prepare_output().complete());
        let velocity = SkeletonRootVector {
            xyz: [1.0, -0.0, 3.0],
            w: None,
        };
        a.publish_prepare_velocity(&own, velocity).unwrap();
        let stored = a.processed.prepare_output().velocity().unwrap();
        assert_eq!(
            stored.value.xyz.map(f32::to_bits),
            velocity.xyz.map(f32::to_bits)
        );
        assert!(stored.value.w.is_none());
        assert_eq!(stored.origin, PhysOutOrigin::Publication);
        assert!(a.processed.prepare_output().complete());
    }
    #[test]
    fn output_reset_has_its_own_full_zero_provenance_and_no_phase_completion() {
        let mut output = SkeletonProcessedPrepareOutput::uninitialized();
        assert!(output.velocity().is_none());
        assert!(output.count().is_none());
        output.reset_source_fields();
        let velocity = output.velocity().unwrap();
        assert_eq!(velocity.origin, PhysOutOrigin::Reset);
        assert_eq!(velocity.value.xyz.map(f32::to_bits), [0; 3]);
        assert_eq!(velocity.value.w.map(f32::to_bits), Some(0));
        assert_eq!(output.count().unwrap().value, 0);
        assert_eq!(output.count().unwrap().origin, PhysOutOrigin::Reset);
        assert!(!output.complete());
    }
}
