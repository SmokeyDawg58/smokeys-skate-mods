//! PhysState_OffBoard_Locomotion.FillPhysOut82d87480 after its manager call.
use super::super::PhysOutAccessError;
use super::{ConcretePhysOutError, PhysOutComponent, PhysicalPlayerPhysOut};
pub(crate) trait LocomotionOutputSource {
    type Error;
    fn grabbing(&mut self) -> Result<bool, Self::Error>;
    fn descriptor(&mut self) -> Result<(i32, i32), Self::Error>;
    fn foot_down(&mut self, foot: usize) -> Result<bool, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum LocomotionOutputError<S> {
    Source(S),
    Access(PhysOutAccessError),
    Component(ConcretePhysOutError<std::convert::Infallible>),
}
impl PhysicalPlayerPhysOut {
    pub(crate) fn try_fill_locomotion_state<S: LocomotionOutputSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), LocomotionOutputError<S::Error>> {
        use LocomotionOutputError as E;
        self.check_current_guard().map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::OffBoard)
            .map_err(E::Component)?;
        self.concrete.offboard.as_mut().unwrap().is_grabbing_object =
            source.grabbing().map_err(E::Source)?;
        self.check_current_guard().map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::State)
            .map_err(E::Component)?;
        let (kind, id) = source.descriptor().map_err(E::Source)?;
        let out = self.concrete.state.as_mut().unwrap();
        out.grab_spline_desc.object_type = kind;
        out.grab_spline_desc.object_id = id;
        for foot in 0..2 {
            let down = source.foot_down(foot).map_err(E::Source)?;
            self.check_current_guard().map_err(E::Access)?;
            self.require_complete_component(PhysOutComponent::OffBoard)
                .map_err(E::Component)?;
            let out = self.concrete.offboard.as_mut().unwrap();
            if foot == 0 {
                out.left_foot_down = down;
            } else {
                out.right_foot_down = down;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::PhysOutAllocation;
    use super::*;
    #[derive(Default)]
    struct Source {
        calls: Vec<usize>,
        fail: bool,
    }
    impl LocomotionOutputSource for Source {
        type Error = ();
        fn grabbing(&mut self) -> Result<bool, ()> {
            self.calls.push(0);
            Ok(true)
        }
        fn descriptor(&mut self) -> Result<(i32, i32), ()> {
            self.calls.push(1);
            Ok((7, 9))
        }
        fn foot_down(&mut self, foot: usize) -> Result<bool, ()> {
            self.calls.push(2 + foot);
            if foot == 1 && self.fail {
                Err(())
            } else {
                Ok(true)
            }
        }
    }
    fn output() -> PhysicalPlayerPhysOut {
        let mut o = PhysicalPlayerPhysOut::new();
        o.try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
            .unwrap();
        o
    }
    #[test]
    fn late_foot_failure_preserves_prior_stores() {
        let mut o = output();
        let mut s = Source {
            fail: true,
            ..Default::default()
        };
        assert!(o.try_fill_locomotion_state(&mut s).is_err());
        assert_eq!(s.calls, [0, 1, 2, 3]);
        assert!(o.concrete.offboard.as_ref().unwrap().is_grabbing_object);
        assert!(o.concrete.offboard.as_ref().unwrap().left_foot_down);
        assert_eq!(
            o.concrete
                .state
                .as_ref()
                .unwrap()
                .grab_spline_desc
                .object_id,
            9
        );
    }
    #[test]
    fn locked_output_does_not_sample_grab_state() {
        let mut o = output();
        o.lock_after_update1().unwrap();
        let mut s = Source::default();
        assert!(matches!(
            o.try_fill_locomotion_state(&mut s),
            Err(LocomotionOutputError::Access(
                PhysOutAccessError::LockedForWrite
            ))
        ));
        assert!(s.calls.is_empty());
    }
}
