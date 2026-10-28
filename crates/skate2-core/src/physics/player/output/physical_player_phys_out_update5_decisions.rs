//! Actual Update5 decision/output span82e05438..55e0, before fresh category.
use super::super::{PhysOutComponent, PhysicalPlayerPhysOut};
use super::prior_input::PriorInputReadError;

pub(crate) trait Update5DecisionOutputSource {
    type Error;
    fn wipeout_requested(&mut self) -> Result<bool, Self::Error>;
    fn runout_requested(&mut self) -> Result<bool, Self::Error>;
    fn heading_adjust_factor(&mut self) -> Result<f32, Self::Error>;
    fn flags_8c4(&mut self, required_mask: u32) -> Result<u32, Self::Error>;
    fn animation_push_speed(&mut self) -> Result<f32, Self::Error>;
    fn offboard_look_x(&mut self) -> Result<f32, Self::Error>;
    fn offboard_look_y(&mut self) -> Result<f32, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physical_player_phys_out::construct::PhysOutAllocation;
    #[derive(Default)]
    struct Source {
        calls: Vec<&'static str>,
        flag_reads: usize,
        fail_runout: bool,
        unknown_flags: u32,
    }
    impl Update5DecisionOutputSource for Source {
        type Error = &'static str;
        fn wipeout_requested(&mut self) -> Result<bool, Self::Error> {
            self.calls.push("wipeout");
            Ok(true)
        }
        fn runout_requested(&mut self) -> Result<bool, Self::Error> {
            self.calls.push("runout");
            if self.fail_runout {
                Err("runout")
            } else {
                Ok(false)
            }
        }
        fn heading_adjust_factor(&mut self) -> Result<f32, Self::Error> {
            self.calls.push("heading");
            Ok(-0.0)
        }
        fn flags_8c4(&mut self, required_mask: u32) -> Result<u32, Self::Error> {
            self.calls.push("flags");
            if self.unknown_flags & required_mask != 0 {
                return Err("unknown flags");
            }
            self.flag_reads += 1;
            Ok(if self.flag_reads == 1 {
                0x0010_0000
            } else {
                0x0200_0000
            })
        }
        fn animation_push_speed(&mut self) -> Result<f32, Self::Error> {
            self.calls.push("push");
            Ok(f32::from_bits(0x7fc0_0017))
        }
        fn offboard_look_x(&mut self) -> Result<f32, Self::Error> {
            self.calls.push("look_x");
            Ok(-2.0)
        }
        fn offboard_look_y(&mut self) -> Result<f32, Self::Error> {
            self.calls.push("look_y");
            Ok(3.0)
        }
    }
    fn output(null: Option<PhysOutComponent>) -> PhysicalPlayerPhysOut {
        let mut output = PhysicalPlayerPhysOut::new();
        output
            .try_construct_complete(|component| {
                Ok::<_, ()>(if Some(component) == null {
                    PhysOutAllocation::Null
                } else {
                    PhysOutAllocation::Host
                })
            })
            .unwrap();
        output
    }
    #[test]
    fn fresh_flags_and_raw_scalars_follow_both_decisions() {
        let mut output = output(None);
        let mut source = Source::default();
        output.try_publish_update5_decisions(&mut source).unwrap();
        assert_eq!(
            source.calls,
            [
                "wipeout", "runout", "heading", "flags", "push", "flags", "look_x", "look_y"
            ]
        );
        let motion = output.concrete.motion.as_ref().unwrap();
        assert!(motion.skateboard_flipped && motion.is_physically_pushing);
        assert_eq!(motion.heading_adjust_factor.to_bits(), 0x8000_0000);
        assert_eq!(motion.anim_push_speed.to_bits(), 0x7fc0_0017);
        assert_eq!(
            output
                .concrete
                .skeleton
                .as_ref()
                .unwrap()
                .off_board_look_at_x
                .to_bits(),
            (-2.0_f32).to_bits()
        );
    }
    #[test]
    fn missing_later_push_bit_preserves_flipped_and_animation_speed() {
        let mut output = output(None);
        let mut source = Source {
            unknown_flags: 0x0200_0000,
            ..Source::default()
        };
        assert!(matches!(
            output.try_publish_update5_decisions(&mut source),
            Err(Update5DecisionOutputError::Source("unknown flags"))
        ));
        let motion = output.concrete.motion.as_ref().unwrap();
        assert!(motion.skateboard_flipped);
        assert_eq!(motion.anim_push_speed.to_bits(), 0x7fc0_0017);
        assert_eq!(
            source.calls,
            ["wipeout", "runout", "heading", "flags", "push", "flags"]
        );
    }
    #[test]
    fn failed_second_decision_keeps_first_request_store() {
        let mut output = output(None);
        let mut source = Source {
            fail_runout: true,
            ..Source::default()
        };
        assert!(matches!(
            output.try_publish_update5_decisions(&mut source),
            Err(Update5DecisionOutputError::Source("runout"))
        ));
        assert!(
            output
                .concrete
                .state
                .as_ref()
                .unwrap()
                .physics_requests_wipeout
        );
        assert_eq!(source.calls, ["wipeout", "runout"]);
    }
    #[test]
    fn state_guard_follows_decision_but_motion_guard_precedes_scalar_read() {
        let mut source = Source::default();
        let mut missing_state = output(Some(PhysOutComponent::State));
        assert!(
            missing_state
                .try_publish_update5_decisions(&mut source)
                .is_err()
        );
        assert_eq!(source.calls, ["wipeout"]);
        let mut source = Source::default();
        let mut missing_motion = output(Some(PhysOutComponent::Motion));
        assert!(
            missing_motion
                .try_publish_update5_decisions(&mut source)
                .is_err()
        );
        assert_eq!(source.calls, ["wipeout", "runout"]);
        assert!(
            missing_motion
                .concrete
                .state
                .as_ref()
                .unwrap()
                .physics_requests_wipeout
        );
    }
}
#[derive(Debug)]
pub(crate) enum Update5DecisionOutputError<E> {
    Source(E),
    Output(PriorInputReadError),
}
impl<E: std::fmt::Debug> std::fmt::Display for Update5DecisionOutputError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(error) => write!(f, "Update5 decision source: {error:?}"),
            Self::Output(error) => write!(f, "Update5 decision output: {error:?}"),
        }
    }
}
impl PhysicalPlayerPhysOut {
    fn decision_output_guard(
        &self,
        component: PhysOutComponent,
    ) -> Result<(), PriorInputReadError> {
        self.check_current_guard()
            .map_err(PriorInputReadError::Access)?;
        self.require_complete_component(component)
            .map_err(PriorInputReadError::Component)
    }
    /// Decision calls precede guards; scalar reads follow guards. Flipped samples
    /// the guard before its flag read; pushing reads its flag before sampling.
    pub(crate) fn try_publish_update5_decisions<S: Update5DecisionOutputSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), Update5DecisionOutputError<S::Error>> {
        use Update5DecisionOutputError as E;
        let wipeout = source.wipeout_requested().map_err(E::Source)?;
        self.decision_output_guard(PhysOutComponent::State)
            .map_err(E::Output)?;
        self.concrete
            .state
            .as_mut()
            .unwrap()
            .physics_requests_wipeout = wipeout;
        let runout = source.runout_requested().map_err(E::Source)?;
        self.decision_output_guard(PhysOutComponent::State)
            .map_err(E::Output)?;
        self.concrete
            .state
            .as_mut()
            .unwrap()
            .physics_requests_runout = runout;
        self.decision_output_guard(PhysOutComponent::Motion)
            .map_err(E::Output)?;
        self.concrete.motion.as_mut().unwrap().heading_adjust_factor =
            source.heading_adjust_factor().map_err(E::Source)?;
        let guard = self
            .sample_write_guard()
            .map_err(|error| E::Output(PriorInputReadError::Access(error)))?;
        let flipped = source.flags_8c4(0x0010_0000).map_err(E::Source)? & 0x0010_0000 != 0;
        self.check_write_guard(&guard)
            .map_err(|error| E::Output(PriorInputReadError::Access(error)))?;
        self.require_complete_component(PhysOutComponent::Motion)
            .map_err(|error| E::Output(PriorInputReadError::Component(error)))?;
        self.concrete.motion.as_mut().unwrap().skateboard_flipped = flipped;
        self.decision_output_guard(PhysOutComponent::Motion)
            .map_err(E::Output)?;
        self.concrete.motion.as_mut().unwrap().anim_push_speed =
            source.animation_push_speed().map_err(E::Source)?;
        let pushing = source.flags_8c4(0x0200_0000).map_err(E::Source)? & 0x0200_0000 != 0;
        self.decision_output_guard(PhysOutComponent::Motion)
            .map_err(E::Output)?;
        self.concrete.motion.as_mut().unwrap().is_physically_pushing = pushing;
        self.decision_output_guard(PhysOutComponent::Skeleton)
            .map_err(E::Output)?;
        self.concrete.skeleton.as_mut().unwrap().off_board_look_at_x =
            source.offboard_look_x().map_err(E::Source)?;
        self.decision_output_guard(PhysOutComponent::Skeleton)
            .map_err(E::Output)?;
        self.concrete.skeleton.as_mut().unwrap().off_board_look_at_y =
            source.offboard_look_y().map_err(E::Source)?;
        Ok(())
    }
}
