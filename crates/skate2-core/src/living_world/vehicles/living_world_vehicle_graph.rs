//! Generated Vehicle.xml, kStates821bd130 (9), kTransitions821bd250 (13),
//! kConditions821bd550 (11), kHooks821bd6a0 (11). Source TemplateHC Update82955fc8.
//! Selected immutable authored graph: all states active, no state expressions,
//! seven noninterruptible sibling leaves under AI/Vehicle. This reduces the
//! original hierarchy searches without substituting a Dynamic controller.
//! Concrete entity callbacks remain required; this does not spawn vehicles.
#[path = "living_world_vehicle_conditions.rs"]
pub(crate) mod conditions;
#[path = "living_world_vehicle_graph_runtime.rs"]
pub(crate) mod runtime;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub(crate) enum VehicleState {
    FollowingLane = 2,
    Impatience = 3,
    ChangingLane = 4,
    PullingOver = 5,
    StayingParked = 6,
    PullingOut = 7,
    PassingIntersection = 8,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub(crate) enum VehicleCondition {
    RequiredToPassIntersection,
    RequiredToRegisterToIntersection,
    RequiredToUnregisterFromIntersection,
    RequiredToBeImpatient,
    RequiredToChangeLane,
    RequiredToPullOver,
    ImpatienceComplete,
    LaneChangePassageComplete,
    RequiredToStopAlarming,
    RequiredToPullOut,
    IntersectionPassageComplete,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub(crate) enum VehicleHook {
    ToPassingIntersection,
    RegisterToIntersection,
    UnregisterFromIntersection,
    ToImpatience,
    ToChangingLane,
    ToPullingOver,
    FromChangingLaneToFollowingLane,
    ToStayingParked,
    StopAlarming,
    ToPullingOut,
    FromPassingIntersectionToFollowingLane,
}
struct Transition {
    target: VehicleState,
    condition: VehicleCondition,
    hook: VehicleHook,
}
const TRANSITIONS: [Transition; 13] = {
    use VehicleCondition as C;
    use VehicleHook as H;
    use VehicleState as S;
    [
        Transition {
            target: S::PassingIntersection,
            condition: C::RequiredToPassIntersection,
            hook: H::ToPassingIntersection,
        },
        Transition {
            target: S::FollowingLane,
            condition: C::RequiredToRegisterToIntersection,
            hook: H::RegisterToIntersection,
        },
        Transition {
            target: S::FollowingLane,
            condition: C::RequiredToUnregisterFromIntersection,
            hook: H::UnregisterFromIntersection,
        },
        Transition {
            target: S::Impatience,
            condition: C::RequiredToBeImpatient,
            hook: H::ToImpatience,
        },
        Transition {
            target: S::ChangingLane,
            condition: C::RequiredToChangeLane,
            hook: H::ToChangingLane,
        },
        Transition {
            target: S::PullingOver,
            condition: C::RequiredToPullOver,
            hook: H::ToPullingOver,
        },
        Transition {
            target: S::ChangingLane,
            condition: C::ImpatienceComplete,
            hook: H::ToChangingLane,
        },
        Transition {
            target: S::FollowingLane,
            condition: C::LaneChangePassageComplete,
            hook: H::FromChangingLaneToFollowingLane,
        },
        Transition {
            target: S::StayingParked,
            condition: C::LaneChangePassageComplete,
            hook: H::ToStayingParked,
        },
        Transition {
            target: S::StayingParked,
            condition: C::RequiredToStopAlarming,
            hook: H::StopAlarming,
        },
        Transition {
            target: S::PullingOut,
            condition: C::RequiredToPullOut,
            hook: H::ToPullingOut,
        },
        Transition {
            target: S::FollowingLane,
            condition: C::LaneChangePassageComplete,
            hook: H::FromChangingLaneToFollowingLane,
        },
        Transition {
            target: S::FollowingLane,
            condition: C::IntersectionPassageComplete,
            hook: H::FromPassingIntersectionToFollowingLane,
        },
    ]
};
impl VehicleState {
    fn transitions(self) -> &'static [Transition] {
        let (start, end) = match self {
            Self::FollowingLane => (0, 6),
            Self::Impatience => (6, 7),
            Self::ChangingLane => (7, 8),
            Self::PullingOver => (8, 9),
            Self::StayingParked => (9, 11),
            Self::PullingOut => (11, 12),
            Self::PassingIntersection => (12, 13),
        };
        &TRANSITIONS[start..end]
    }
}
/// Read view is captured at each callback boundary, never before the whole tick.
#[derive(Clone, Copy)]
pub(crate) struct VehicleGraphView<'a> {
    pub(crate) current: Option<VehicleState>,
    pub(crate) last: Option<VehicleState>,
    pub(crate) delta_time: f32,
    times: &'a [Option<f32>; 9],
}
impl VehicleGraphView<'_> {
    pub(crate) fn time_in_state(self, state: u16) -> f32 {
        self.times
            .get(state as usize)
            .copied()
            .flatten()
            .unwrap_or(-1.)
    }
}
pub(crate) trait VehicleGraphRuntime {
    type Error;
    fn condition(
        &mut self,
        condition: VehicleCondition,
        view: VehicleGraphView<'_>,
    ) -> Result<bool, Self::Error>;
    fn hook(&mut self, hook: VehicleHook, view: VehicleGraphView<'_>) -> Result<(), Self::Error>;
    fn begin(&mut self, state: VehicleState, view: VehicleGraphView<'_>)
    -> Result<(), Self::Error>;
    fn end(&mut self, state: VehicleState, view: VehicleGraphView<'_>) -> Result<(), Self::Error>;
    fn update(
        &mut self,
        state: VehicleState,
        view: VehicleGraphView<'_>,
    ) -> Result<(), Self::Error>;
}
pub(crate) struct VehicleGraph {
    current: Option<VehicleState>,
    last: Option<VehicleState>,
    /// One authored leaf behaviour; ancestors AI/Vehicle have none. Kept
    /// separate because EndAllBehaviours does not clear the selected state.
    active_behaviour: Option<VehicleState>,
    delta_time: f32,
    times: [Option<f32>; 9],
    call_behaviours: bool,
}
impl VehicleGraph {
    /// Constructor8292c510. All seven CreateInstance callbacks82bb3da8 return
    /// null; retain no fabricated per-behaviour state object.
    pub(crate) fn new(call_behaviours: bool) -> Self {
        Self {
            current: None,
            last: None,
            active_behaviour: None,
            delta_time: 0.,
            times: [None; 9],
            call_behaviours,
        }
    }
    pub(crate) fn view(&self) -> VehicleGraphView<'_> {
        VehicleGraphView {
            current: self.current,
            last: self.last,
            delta_time: self.delta_time,
            times: &self.times,
        }
    }
    /// TemplateHC<Vehicle>.EndAllBehaviours8292e2e8: End, then erase its
    /// behaviour instance. Current/last state, delta time and clocks survive.
    pub(crate) fn end_all_behaviours<R: VehicleGraphRuntime>(
        &mut self,
        runtime: &mut R,
    ) -> Result<(), R::Error> {
        if let Some(state) = self.active_behaviour {
            if self.call_behaviours {
                runtime.end(state, self.view())?;
            }
            self.active_behaviour = None;
        }
        Ok(())
    }
    pub(crate) fn update<R: VehicleGraphRuntime>(
        &mut self,
        dt: f32,
        runtime: &mut R,
    ) -> Result<(), R::Error> {
        self.delta_time = dt;
        let old = self.current;
        let mut selected = None;
        if let Some(state) = old {
            // SearchTransitions8292e608/8292e098: all13 priorities are zero,
            // so priority3/2/1 scans call no predicates. First true wins.
            for transition in state.transitions() {
                if runtime.condition(transition.condition, self.view())? {
                    selected = Some(transition);
                    break;
                }
            }
        }
        let next = selected.map_or(old.unwrap_or(VehicleState::FollowingLane), |t| t.target);
        // UpdateLists ages old time entries BEFORE publishing current/last.
        for time in self.times.iter_mut().flatten() {
            *time = dt + *time;
        }
        self.last = old;
        self.current = Some(next);
        // Every authored transition targets a leaf. Even self-transitions
        // leave/reenter that leaf, keeping AI/Vehicle ancestor clocks intact.
        if selected.is_some() {
            if let Some(state) = old {
                // UpdateExitStates8292e408 walks active behaviours first;
                // an earlier EndAllBehaviours must not issue End twice.
                self.end_all_behaviours(runtime)?;
                self.times[state as usize] = None;
            }
        }
        // Hooks execute even when mCallBehaviours is false.
        if let Some(transition) = selected {
            runtime.hook(transition.hook, self.view())?;
        }
        if old.is_none() || selected.is_some() {
            if old.is_none() {
                self.times[0] = Some(0.);
                self.times[1] = Some(0.);
            }
            self.times[next as usize] = Some(0.);
            // UpdateEnterStates82952ef8 publishes the instance BEFORE Begin.
            self.active_behaviour = Some(next);
            if self.call_behaviours {
                runtime.begin(next, self.view())?;
            }
        }
        if self.call_behaviours {
            // Generated UpdateBehaviours8292cf40 dispatches kStates[current]
            // even if EndAllBehaviours has emptied its instance list.
            runtime.update(next, self.view())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Probe {
        true_conditions: Vec<VehicleCondition>,
        queried: Vec<VehicleCondition>,
        events: Vec<(&'static str, Option<VehicleState>, f32, f32)>,
    }
    impl Probe {
        fn event(&mut self, name: &'static str, view: VehicleGraphView<'_>) {
            let time = view.current.map_or(-1., |s| view.time_in_state(s as u16));
            self.events
                .push((name, view.current, time, view.time_in_state(0)));
        }
    }
    impl VehicleGraphRuntime for Probe {
        type Error = ();
        fn condition(&mut self, c: VehicleCondition, _: VehicleGraphView<'_>) -> Result<bool, ()> {
            self.queried.push(c);
            Ok(self.true_conditions.contains(&c))
        }
        fn hook(&mut self, _: VehicleHook, v: VehicleGraphView<'_>) -> Result<(), ()> {
            self.event("hook", v);
            Ok(())
        }
        fn begin(&mut self, _: VehicleState, v: VehicleGraphView<'_>) -> Result<(), ()> {
            self.event("begin", v);
            Ok(())
        }
        fn end(&mut self, _: VehicleState, v: VehicleGraphView<'_>) -> Result<(), ()> {
            self.event("end", v);
            Ok(())
        }
        fn update(&mut self, _: VehicleState, v: VehicleGraphView<'_>) -> Result<(), ()> {
            self.event("update", v);
            Ok(())
        }
    }
    #[test]
    fn end_all_is_idempotent_and_retains_state_clocks_and_generated_update() {
        let mut graph = VehicleGraph::new(true);
        let mut p = Probe::default();
        graph.update(0.25, &mut p).unwrap();
        graph.update(0.25, &mut p).unwrap();
        p.events.clear();
        graph.end_all_behaviours(&mut p).unwrap();
        graph.end_all_behaviours(&mut p).unwrap();
        assert_eq!(
            p.events,
            vec![("end", Some(VehicleState::FollowingLane), 0.25, 0.25)]
        );
        assert_eq!(graph.view().current, Some(VehicleState::FollowingLane));
        assert_eq!(graph.view().time_in_state(2), 0.25);
        p.events.clear();
        graph.update(0.25, &mut p).unwrap();
        assert_eq!(
            p.events,
            vec![("update", Some(VehicleState::FollowingLane), 0.5, 0.5)]
        );
        p.events.clear();
        p.true_conditions
            .push(VehicleCondition::RequiredToBeImpatient);
        graph.update(0.25, &mut p).unwrap();
        assert_eq!(
            p.events.iter().map(|event| event.0).collect::<Vec<_>>(),
            vec!["hook", "begin", "update"]
        );
        assert_eq!(graph.active_behaviour, Some(VehicleState::Impatience));
    }

    #[test]
    fn self_transition_restarts_leaf_but_preserves_ancestor_clock_and_hook_order() {
        let mut graph = VehicleGraph::new(true);
        let mut p = Probe::default();
        graph.update(0.25, &mut p).unwrap();
        assert!(p.queried.is_empty());
        graph.update(0.25, &mut p).unwrap();
        p.events.clear();
        p.queried.clear();
        p.true_conditions = vec![
            VehicleCondition::RequiredToRegisterToIntersection,
            VehicleCondition::RequiredToBeImpatient,
        ];
        graph.update(0.25, &mut p).unwrap();
        assert_eq!(
            p.queried,
            vec![
                VehicleCondition::RequiredToPassIntersection,
                VehicleCondition::RequiredToRegisterToIntersection
            ]
        );
        let state = Some(VehicleState::FollowingLane);
        assert_eq!(
            p.events,
            vec![
                ("end", state, 0.5, 0.5),
                ("hook", state, -1., 0.5),
                ("begin", state, 0., 0.5),
                ("update", state, 0., 0.5)
            ]
        );
    }
    #[test]
    fn disabling_behaviour_calls_does_not_disable_transition_hooks() {
        let mut graph = VehicleGraph::new(false);
        let mut p = Probe::default();
        graph.update(0.25, &mut p).unwrap();
        p.true_conditions
            .push(VehicleCondition::RequiredToPassIntersection);
        graph.update(0.25, &mut p).unwrap();
        assert_eq!(
            graph.view().current,
            Some(VehicleState::PassingIntersection)
        );
        assert_eq!(
            p.events,
            vec![("hook", Some(VehicleState::PassingIntersection), -1., 0.25)]
        );
    }
}
