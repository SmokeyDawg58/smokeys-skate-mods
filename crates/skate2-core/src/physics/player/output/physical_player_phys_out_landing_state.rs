//! State505 FillPhysOut82d89898: manager fill is scheduled after this method.
use super::super::PhysOutAccessError;
use super::{ConcretePhysOutError, PhysOutComponent, PhysicalPlayerPhysOut};
pub(crate) trait LandingOutputSource {
    type Error;
    fn wipeout_time(&mut self) -> Result<f32, Self::Error>;
    fn time_to_land(&mut self) -> Result<f32, Self::Error>;
    fn on_deck(&mut self) -> Result<bool, Self::Error>;
    fn just_hippy(&mut self) -> Result<bool, Self::Error>;
    fn flip(&mut self) -> Result<bool, Self::Error>;
    fn hippy(&mut self) -> Result<bool, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum LandingOutputError<S> {
    Source(S),
    Access(PhysOutAccessError),
    Component(ConcretePhysOutError<std::convert::Infallible>),
}
impl PhysicalPlayerPhysOut {
    pub(crate) fn try_fill_landing_state<S: LandingOutputSource>(
        &mut self,
        s: &mut S,
    ) -> Result<(), LandingOutputError<S::Error>> {
        use LandingOutputError as E;
        let guard = self.sample_write_guard().map_err(E::Access)?;
        let wipeout = s.wipeout_time().map_err(E::Source)?;
        self.check_write_guard(&guard).map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::OffBoard)
            .map_err(E::Component)?;
        self.concrete.offboard.as_mut().unwrap().time_to_land =
            s.time_to_land().map_err(E::Source)?;
        self.check_current_guard().map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::OffBoard)
            .map_err(E::Component)?;
        self.concrete.offboard.as_mut().unwrap().on_deck = s.on_deck().map_err(E::Source)?;
        let hippy = s.just_hippy().map_err(E::Source)?;
        self.check_current_guard().map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::Ground)
            .map_err(E::Component)?;
        self.concrete.ground.as_mut().unwrap().just_hippy_jumped = hippy;
        self.check_current_guard().map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::OffBoard)
            .map_err(E::Component)?;
        self.concrete
            .offboard
            .as_mut()
            .unwrap()
            .land_on_skateboard_flip_board = s.flip().map_err(E::Source)?;
        self.check_current_guard().map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::Ground)
            .map_err(E::Component)?;
        self.concrete.ground.as_mut().unwrap().is_full_hippy_jump = s.hippy().map_err(E::Source)?;
        if s.flip().map_err(E::Source)? && s.time_to_land().map_err(E::Source)? < wipeout {
            self.check_current_guard().map_err(E::Access)?;
            self.require_complete_component(PhysOutComponent::State)
                .map_err(E::Component)?;
            self.concrete
                .state
                .as_mut()
                .unwrap()
                .phys_state_wants_wipeout = true;
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
        calls: Vec<&'static str>,
        fail: bool,
    }
    impl LandingOutputSource for Source {
        type Error = ();
        fn wipeout_time(&mut self) -> Result<f32, ()> {
            self.calls.push("limit");
            Ok(0.02)
        }
        fn time_to_land(&mut self) -> Result<f32, ()> {
            self.calls.push("ttl");
            Ok(0.01)
        }
        fn on_deck(&mut self) -> Result<bool, ()> {
            self.calls.push("deck");
            Ok(true)
        }
        fn just_hippy(&mut self) -> Result<bool, ()> {
            self.calls.push("just");
            Ok(true)
        }
        fn flip(&mut self) -> Result<bool, ()> {
            self.calls.push("flip");
            if self.fail { Err(()) } else { Ok(true) }
        }
        fn hippy(&mut self) -> Result<bool, ()> {
            self.calls.push("hippy");
            Ok(true)
        }
    }
    fn output() -> PhysicalPlayerPhysOut {
        let mut o = PhysicalPlayerPhysOut::new();
        o.try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
            .unwrap();
        o
    }
    #[test]
    fn ordered_output_keeps_completed_prefix_on_late_failure() {
        let mut o = output();
        let mut s = Source {
            fail: true,
            ..Default::default()
        };
        assert!(o.try_fill_landing_state(&mut s).is_err());
        assert_eq!(s.calls, ["limit", "ttl", "deck", "just", "flip"]);
        assert_eq!(o.concrete.offboard.as_ref().unwrap().time_to_land, 0.01);
        assert!(o.concrete.offboard.as_ref().unwrap().on_deck);
        assert!(o.concrete.ground.as_ref().unwrap().just_hippy_jumped);
        assert!(!o.concrete.state.as_ref().unwrap().phys_state_wants_wipeout);
        s.fail = false;
        s.calls.clear();
        o.try_fill_landing_state(&mut s).unwrap();
        assert_eq!(
            s.calls,
            [
                "limit", "ttl", "deck", "just", "flip", "hippy", "flip", "ttl"
            ]
        );
        assert!(o.concrete.state.as_ref().unwrap().phys_state_wants_wipeout);
    }
    #[test]
    fn locked_output_reads_limit_before_assert_but_no_state_value() {
        let mut o = output();
        o.lock_after_update1().unwrap();
        let mut s = Source::default();
        assert!(matches!(
            o.try_fill_landing_state(&mut s),
            Err(LandingOutputError::Access(
                PhysOutAccessError::LockedForWrite
            ))
        ));
        assert_eq!(s.calls, ["limit"]);
    }
}
