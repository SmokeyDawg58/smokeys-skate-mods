//! Selected Skeleton `FillPhysOut` COM copies (0x82b585ac..0x82b58614).
use super::{PhysOutPublicationError, PhysOutSystemVector, PhysicalPlayerPhysOut};
use crate::skeleton_root_transforms::SkeletonRootVector;

pub(crate) trait PhysOutComSource {
    type Error;
    fn centre_of_mass_velocity(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn centre_of_mass(&mut self) -> Result<SkeletonRootVector, Self::Error>;
}
impl PhysicalPlayerPhysOut {
    /// The velocity load precedes the first latch assertion; COM follows it.
    /// A second assertion guards the retained early velocity's store.
    pub(crate) fn try_publish_skeleton_com<S: PhysOutComSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), PhysOutPublicationError<S::Error>> {
        self.try_publish_skeleton_com_capture(source).map(|_| ())
    }
    /// Return the original585b0 velocity after both guarded COM stores.
    pub(crate) fn try_publish_skeleton_com_capture<S: PhysOutComSource>(
        &mut self,
        source: &mut S,
    ) -> Result<SkeletonRootVector, PhysOutPublicationError<S::Error>> {
        use PhysOutPublicationError::{Access, Source};
        let guard = self.sample_write_guard().map_err(Access)?;
        let velocity = source.centre_of_mass_velocity().map_err(Source)?;
        self.check_write_guard(&guard).map_err(Access)?;
        let centre = source.centre_of_mass().map_err(Source)?;
        self.publish_system_vector(&guard, PhysOutSystemVector::CentreOfMass, centre)
            .map_err(Access)?;
        let guard = self.sample_write_guard().map_err(Access)?;
        self.publish_system_vector(&guard, PhysOutSystemVector::CentreOfMassVelocity, velocity)
            .map_err(Access)?;
        Ok(velocity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physical_player_phys_out::{
        PhysOutAccessError, PhysOutOrigin, PhysOutRemainder, PhysOutRemainderStage,
    };
    struct Remainder;
    impl PhysOutRemainder for Remainder {
        type Error = ();
        fn run(&mut self, _: PhysOutRemainderStage) -> Result<(), ()> {
            Ok(())
        }
    }
    #[derive(Default)]
    struct Source {
        reads: Vec<&'static str>,
        fail: bool,
    }
    impl PhysOutComSource for Source {
        type Error = ();
        fn centre_of_mass_velocity(&mut self) -> Result<SkeletonRootVector, ()> {
            self.reads.push("velocity");
            Ok(SkeletonRootVector::known([3.0, -0.0, 5.0, 7.0]))
        }
        fn centre_of_mass(&mut self) -> Result<SkeletonRootVector, ()> {
            self.reads.push("centre");
            if self.fail {
                Err(())
            } else {
                Ok(SkeletonRootVector::known([8.0, 9.0, 10.0, 11.0]))
            }
        }
    }
    #[test]
    fn sampled_locked_latch_still_reads_early_velocity_only() {
        let mut output = PhysicalPlayerPhysOut::new();
        output.try_construct(&mut Remainder).unwrap();
        output.lock_after_update1().unwrap();
        let mut source = Source::default();
        assert_eq!(
            output.try_publish_skeleton_com(&mut source),
            Err(PhysOutPublicationError::Access(
                PhysOutAccessError::LockedForWrite
            ))
        );
        assert_eq!(source.reads, ["velocity"]);
    }
    #[test]
    fn late_source_failure_preserves_constructor_then_full_lanes_publish() {
        let mut output = PhysicalPlayerPhysOut::new();
        output.try_construct(&mut Remainder).unwrap();
        let mut source = Source {
            fail: true,
            ..Source::default()
        };
        assert_eq!(
            output.try_publish_skeleton_com(&mut source),
            Err(PhysOutPublicationError::Source(()))
        );
        for field in [
            PhysOutSystemVector::CentreOfMass,
            PhysOutSystemVector::CentreOfMassVelocity,
        ] {
            let old = output.system_vector(field).unwrap();
            assert_eq!(old.origin, PhysOutOrigin::Constructor);
            assert_eq!(old.value.w, Some(0.0));
        }
        source.fail = false;
        output.try_publish_skeleton_com(&mut source).unwrap();
        assert_eq!(source.reads, ["velocity", "centre", "velocity", "centre"]);
        let velocity = output
            .system_vector(PhysOutSystemVector::CentreOfMassVelocity)
            .unwrap();
        assert_eq!(velocity.origin, PhysOutOrigin::Publication);
        assert_eq!(velocity.value.xyz[1].to_bits(), (-0.0_f32).to_bits());
        assert_eq!(velocity.value.w, Some(7.0));
        assert_eq!(
            output
                .system_vector(PhysOutSystemVector::CentreOfMass)
                .unwrap()
                .value
                .w,
            Some(11.0)
        );
    }
}

#[cfg(test)]
#[path = "physical_player_phys_out_com_capture_tests.rs"]
mod capture_tests;
