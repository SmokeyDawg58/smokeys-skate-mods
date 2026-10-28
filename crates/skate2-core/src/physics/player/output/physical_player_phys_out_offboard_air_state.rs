//! State502 FillPhysOut82d94510 after the LandingOnDeck manager child.
use super::super::PhysOutAccessError;
use super::{ConcretePhysOutError, PhysOutComponent, PhysicalPlayerPhysOut};

pub(crate) trait AirOutputSource {
    type Error;
    fn time_to_land(&mut self) -> Result<f32, Self::Error>;
    fn landing_velocity_y(&mut self) -> Result<f32, Self::Error>;
    fn trajectory_ready(&mut self) -> Result<bool, Self::Error>;
    fn has_landed_for_locomotion(&mut self) -> Result<bool, Self::Error>;
    fn foot_down(&mut self, left: bool) -> Result<bool, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum AirOutputError<S> {
    Source(S),
    Access(PhysOutAccessError),
    Component(ConcretePhysOutError<std::convert::Infallible>),
}
impl PhysicalPlayerPhysOut {
    pub(crate) fn try_fill_offboard_air_state<S: AirOutputSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), AirOutputError<S::Error>> {
        use AirOutputError as E;
        self.check_current_guard().map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::OffBoard)
            .map_err(E::Component)?;
        self.concrete.offboard.as_mut().unwrap().time_to_land =
            source.time_to_land().map_err(E::Source)?;
        let guard = self.sample_write_guard().map_err(E::Access)?;
        let velocity = source.landing_velocity_y().map_err(E::Source)?;
        self.check_write_guard(&guard).map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::OffBoard)
            .map_err(E::Component)?;
        self.concrete.offboard.as_mut().unwrap().landing_velocity = velocity;
        let guard = self.sample_write_guard().map_err(E::Access)?;
        let ready = source.trajectory_ready().map_err(E::Source)?;
        self.check_write_guard(&guard).map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::OffBoard)
            .map_err(E::Component)?;
        self.concrete.offboard.as_mut().unwrap().trajectory_ready = ready;
        let landed = source.has_landed_for_locomotion().map_err(E::Source)?;
        self.check_current_guard().map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::OffBoard)
            .map_err(E::Component)?;
        self.concrete.offboard.as_mut().unwrap().has_landed = landed;
        for left in [true, false] {
            self.check_current_guard().map_err(E::Access)?;
            self.require_complete_component(PhysOutComponent::OffBoard)
                .map_err(E::Component)?;
            let down = source.foot_down(left).map_err(E::Source)?;
            let out = self.concrete.offboard.as_mut().unwrap();
            if left {
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
        calls: Vec<&'static str>,
        fail: bool,
    }
    impl AirOutputSource for Source {
        type Error = ();
        fn time_to_land(&mut self) -> Result<f32, ()> {
            self.calls.push("ttl");
            Ok(2.5)
        }
        fn landing_velocity_y(&mut self) -> Result<f32, ()> {
            self.calls.push("velocity");
            Ok(-3.0)
        }
        fn trajectory_ready(&mut self) -> Result<bool, ()> {
            self.calls.push("ready");
            Ok(true)
        }
        fn has_landed_for_locomotion(&mut self) -> Result<bool, ()> {
            self.calls.push("landed");
            if self.fail { Err(()) } else { Ok(true) }
        }
        fn foot_down(&mut self, left: bool) -> Result<bool, ()> {
            self.calls.push(if left { "left" } else { "right" });
            Ok(left)
        }
    }
    fn output() -> PhysicalPlayerPhysOut {
        let mut out = PhysicalPlayerPhysOut::new();
        out.try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
            .unwrap();
        out
    }
    #[test]
    fn writes_real_component_in_order_and_preserves_prefix_on_late_failure() {
        let mut out = output();
        let mut s = Source {
            fail: true,
            ..Default::default()
        };
        assert!(matches!(
            out.try_fill_offboard_air_state(&mut s),
            Err(AirOutputError::Source(()))
        ));
        assert_eq!(s.calls, ["ttl", "velocity", "ready", "landed"]);
        let v = out.concrete.offboard.as_ref().unwrap();
        assert_eq!(v.time_to_land, 2.5);
        assert_eq!(v.landing_velocity, -3.0);
        assert!(v.trajectory_ready);
        s.fail = false;
        s.calls.clear();
        out.try_fill_offboard_air_state(&mut s).unwrap();
        assert_eq!(
            s.calls,
            ["ttl", "velocity", "ready", "landed", "left", "right"]
        );
        let v = out.concrete.offboard.as_ref().unwrap();
        assert!(v.has_landed && v.left_foot_down && !v.right_foot_down);
    }
    #[test]
    fn locked_output_refuses_before_first_state_read() {
        let mut out = output();
        out.lock_after_update1().unwrap();
        let mut s = Source::default();
        assert!(matches!(
            out.try_fill_offboard_air_state(&mut s),
            Err(AirOutputError::Access(PhysOutAccessError::LockedForWrite))
        ));
        assert!(s.calls.is_empty());
    }
}
