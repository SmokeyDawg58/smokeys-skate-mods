//! HCDelegate_MotionGraph_Pedestrian lifecycle829562e8. This is the pedestrian
//! controller, never the onboard controller. Expressions and typed conditions
//! are retained; live condition services and behavior callbacks remain required.
//! The marquee entities' HCDelegate_motiongraph_marquee and
//! HCDelegate_Motiongraph_Photographer run on the same lifecycle.
use crate::{
    dynamic_state_graph_controller::DynamicStateGraphContext,
    generated_action_load::{Attribute, BehaviourDeclaration, ConditionDeclaration, NULL_INDEX,
        StateDeclaration, TransitionDeclaration, runtime::ActionUpdateError as Error},
    generated_graph_selection::{Graph, Mask},
    motion_graph_entity_authority::{ActorGraphAuthority, BoundMotionGraphContext,
        MotionGraphControllerToken, MotionGraphInstanceToken},
    state_graph_context::{StateGraphContext, StateGraphContextHandle},
};
use std::collections::BTreeMap;
#[path = "pedestrian_motion_data.rs"]
pub(crate) mod data;
#[path = "pedestrian_motion_ids.rs"]
pub(crate) mod ids;
#[path = "pedestrian_motion_expressions.rs"]
pub(crate) mod expressions;
#[path = "pedestrian_motion_conditions.rs"]
pub(crate) mod conditions;
#[path = "pedestrian_motion_behaviours.rs"]
pub(crate) mod behaviours;
#[path = "pedestrian_motion_backend.rs"]
pub(crate) mod backend;
#[path = "pedestrian_motion_live.rs"]
pub(crate) mod live;
#[path = "pedestrian_motion_world.rs"]
mod world;
#[path = "pedestrian_motion_channel.rs"]
pub(crate) mod channel;
#[path = "pedestrian_motion_frame.rs"]
pub(crate) mod frame;
#[path = "pedestrian_motion_pose.rs"]
pub(crate) mod pose;
#[path = "pedestrian_marquee_motion_data.rs"]
pub(crate) mod marquee_data;

/// The generated motion delegates a pedestrian's animation data runs:
/// `GENERATED_STATE_GRAPH_DELEGATE_BASENAMES` 4 (`MotionGraph_Pedestrian.xml`), 17
/// (`motiongraph_marquee.xml`) and 18 (`Motiongraph_Photographer.xml`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MotionDelegate {
    Pedestrian,
    Marquee,
    Photographer,
}

/// A motion delegate's declarations and its behaviour instance capacity.
pub(crate) struct MotionDeclarations {
    pub(crate) states: &'static [StateDeclaration],
    pub(crate) transitions: &'static [TransitionDeclaration],
    pub(crate) indices: &'static [u16],
    pub(crate) behaviours: &'static [BehaviourDeclaration],
    pub(crate) conditions: &'static [ConditionDeclaration],
    pub(crate) attributes: &'static [Attribute],
    pub(crate) active_capacity: usize,
}

impl MotionDelegate {
    pub(crate) const fn from_delegate_index(index: usize) -> Option<Self> {
        match index {
            4 => Some(Self::Pedestrian),
            17 => Some(Self::Marquee),
            18 => Some(Self::Photographer),
            _ => None,
        }
    }

    /// HCDelegate_motiongraph_marquee (TemplateHC constructor 0x82949300) and
    /// HCDelegate_Motiongraph_Photographer (0x8294ae88) hold three behaviour instances.
    pub(crate) fn declarations(self) -> MotionDeclarations {
        use marquee_data::{marquee, photographer};
        match self {
            Self::Pedestrian => MotionDeclarations { states: &data::STATES, transitions: &data::TRANSITIONS,
                indices: &data::INDICES, behaviours: &data::BEHAVIOURS, conditions: &data::CONDITIONS,
                attributes: &data::ATTRIBUTES, active_capacity: 7 },
            Self::Marquee => MotionDeclarations { states: &marquee::STATES, transitions: &marquee::TRANSITIONS,
                indices: &marquee::INDICES, behaviours: &marquee::BEHAVIOURS, conditions: &marquee::CONDITIONS,
                attributes: &marquee::ATTRIBUTES, active_capacity: 3 },
            Self::Photographer => MotionDeclarations { states: &photographer::STATES,
                transitions: &photographer::TRANSITIONS, indices: &photographer::INDICES,
                behaviours: &photographer::BEHAVIOURS, conditions: &photographer::CONDITIONS,
                attributes: &photographer::ATTRIBUTES, active_capacity: 3 },
        }
    }
}

/// Native 128-bit active/value pair. A condition is cached by declaration index,
/// not expression, mask or state. Failed host callbacks never publish a value.
#[derive(Default)]
pub(crate) struct ConditionResults { active: [u32; 4], values: [u32; 4] }
impl ConditionResults {
    fn begin_frame(&mut self) { self.active = [0; 4]; }
    pub(crate) fn evaluate(&mut self, condition: u16,
        callback: impl FnOnce() -> Result<bool, Error>) -> Result<bool, Error>
    {
        if usize::from(condition) >= data::CONDITIONS.len() {
            return Err(Error::InvalidActiveSet);
        }
        let word = usize::from(condition / 32);
        let bit = 1_u32 << (condition % 32);
        if self.active[word] & bit != 0 { return Ok(self.values[word] & bit != 0); }
        let value = callback()?;
        self.active[word] |= bit;
        self.values[word] = (self.values[word] & !bit) | if value { bit } else { 0 };
        Ok(value)
    }
}

pub(crate) struct ActiveBehaviour {
    pub(crate) occurrence: u16,
    pub(crate) instance: Option<MotionGraphInstanceToken>,
}
pub(crate) struct ControllerState<'a> {
    pub(crate) current: u16,
    pub(crate) last: u16,
    pub(crate) delta_time: f32,
    pub(crate) times: &'a BTreeMap<u16, f32>,
}
pub(crate) struct PedestrianMotionDelegate<K> {
    pub(crate) delegate: MotionDelegate,
    pub(crate) current: u16,
    pub(crate) last: u16,
    pub(crate) delta_time: f32,
    pub(crate) call_behaviours: bool,
    context: BoundMotionGraphContext<K>,
    times: BTreeMap<u16, f32>,
    active: Vec<ActiveBehaviour>,
    condition_results: ConditionResults,
}

/// Implement against the existing pedestrian animation/intent owners. Load must
/// bind Conditions and all native behavior objects before creating this runner.
/// Missing callbacks must return their exact pending-source error.
pub(crate) trait PedestrianMotionBackend<K> {
    fn condition(&mut self, context: &BoundMotionGraphContext<K>,
        controller: &ControllerState<'_>, condition: u16, state: u16) -> Result<bool, Error>;
    fn enabled(&self, occurrence: u16) -> Result<bool, Error>;
    fn create_instance(&mut self, occurrence: u16)
        -> Result<Option<MotionGraphInstanceToken>, Error>;
    fn begin(&mut self, context: &BoundMotionGraphContext<K>, controller: &ControllerState<'_>, occurrence: u16) -> Result<(), Error>;
    fn end(&mut self, context: &BoundMotionGraphContext<K>, controller: &ControllerState<'_>, occurrence: u16) -> Result<(), Error>;
    fn release_instance(&mut self, occurrence: u16,
        instance: Option<MotionGraphInstanceToken>) -> Result<(), Error>;
    /// Native direct StateUpdate mutates the original Context instance slot.
    /// This method must follow the selected source body's actual dispatch order;
    /// calling every active virtual Update would not implement that body.
    fn generated_update(&mut self, context: &mut BoundMotionGraphContext<K>,
        controller: &ControllerState<'_>, source: u32, active: &[ActiveBehaviour]) -> Result<(), Error>;
}

impl<K> PedestrianMotionDelegate<K> {
    /// TemplateHC82930ce8 takes call_behaviours from its actual load caller.
    pub(crate) fn new(authority: &ActorGraphAuthority<K>, call_behaviours: bool, delegate: MotionDelegate) -> Self {
        Self {
            delegate, current: NULL_INDEX, last: NULL_INDEX, delta_time: 0., call_behaviours,
            context: StateGraphContext::new(Some(MotionGraphControllerToken::issue()),
                Some(authority.entity_handle()), None, None, None, None),
            times: BTreeMap::new(), active: Vec::new(), condition_results: ConditionResults::default(),
        }
    }
    pub(crate) fn context(&self) -> &BoundMotionGraphContext<K> { &self.context }
    pub(crate) fn time_in_state(&self, state: u16) -> f32 {
        self.times.get(&state).copied().unwrap_or(-1.)
    }
    /// Update829562e8 / Lists82952368 / Exit82932360 / Enter82953278 on the delegate's
    /// declarations: `IndexSet<7>` for MotionGraph_Pedestrian and `IndexSet<6>` for the
    /// marquee delegates (Update 0x82957748, 0x829578d8). Only MotionGraph_Pedestrian caches
    /// its conditions per frame; the marquee evaluators call them directly.
    pub(crate) fn update(&mut self, dt: f32,
        backend: &mut impl PedestrianMotionBackend<K>) -> Result<(), Error>
    {
        let declarations = self.delegate.declarations();
        match self.delegate {
            MotionDelegate::Pedestrian => self.update_declarations::<7>(dt, backend, &declarations,
                |mask, expression, mut condition| match mask {
                    Mask::Pre => expressions::precondition(expression, &mut condition),
                    Mask::Sustain => sustain(expression, &mut condition),
                }),
            MotionDelegate::Marquee => self.update_declarations::<6>(dt, backend, &declarations,
                |mask, expression, mut condition| expressions::evaluate_marquee(mask, expression, &mut condition)),
            MotionDelegate::Photographer => self.update_declarations::<6>(dt, backend, &declarations,
                |mask, expression, mut condition| expressions::evaluate_photographer(mask, expression, &mut condition)),
        }
    }
    fn update_declarations<const DEPTH: usize>(&mut self, dt: f32,
        backend: &mut impl PedestrianMotionBackend<K>, declarations: &MotionDeclarations,
        evaluate: impl Fn(Mask, u16, &mut dyn FnMut(u16) -> Result<bool, Error>) -> Result<bool, Error>,
    ) -> Result<(), Error> {
        let cached = self.delegate == MotionDelegate::Pedestrian;
        self.condition_results.begin_frame();
        self.delta_time = dt;
        let graph = Graph::<DEPTH> { states: declarations.states, transitions: declarations.transitions,
            indices: declarations.indices, current: self.current };
        let controller = ControllerState { current: self.current, last: self.last,
            delta_time: dt, times: &self.times };
        let selected = graph.next(&mut |mask, expression, state| evaluate(mask, expression, &mut |condition|
            if cached {
                self.condition_results.evaluate(condition, || backend.condition(&self.context, &controller, condition, state))
            } else {
                backend.condition(&self.context, &controller, condition, state)
            }))?;
        if selected.state == NULL_INDEX { return Err(Error::NoSelectedState); }
        let (exited, entered) = graph.changes(&selected)?;
        for time in self.times.values_mut() { *time += dt; }
        self.last = self.current;
        self.current = selected.state;
        for position in (0..self.active.len()).rev() {
            let active = &self.active[position];
            let parent = declarations.behaviours[usize::from(active.occurrence)].parent;
            if parent != NULL_INDEX && exited.contains(&parent) {
                if self.call_behaviours {
                    let mut context = self.context.copy_for_behaviour();
                    context.set_generated_instance(active.instance.as_ref().map(StateGraphContextHandle::shallow_copy));
                    backend.end(&context, &ControllerState { current: self.current, last: self.last,
                        delta_time: dt, times: &self.times }, active.occurrence)?;
                }
                let active = self.active.remove(position);
                backend.release_instance(active.occurrence, active.instance)?;
            }
        }
        for state in exited {
            if self.times.remove(&state).is_none() { return Err(Error::InvalidActiveSet); }
        }
        // Transition829318f8: every authored hook section of the three delegates is empty.
        if selected.transition != NULL_INDEX && declarations.transitions[usize::from(selected.transition)].hooks.count != 0 {
            return Err(Error::PendingTransitionHook(selected.transition));
        }
        for state in entered {
            if !self.times.contains_key(&state) && self.times.len() == DEPTH { return Err(Error::Capacity); }
            self.times.insert(state, 0.);
            let section = declarations.states[usize::from(state)].behaviours;
            for occurrence in section.index..section.index + section.count {
                if !backend.enabled(occurrence)? { continue; }
                let position = self.active.binary_search_by_key(&occurrence, |x| x.occurrence)
                    .map_or_else(|position| Ok(position), |_| Err(Error::InvalidActiveSet))?;
                if self.active.len() == declarations.active_capacity { return Err(Error::Capacity); }
                let instance = backend.create_instance(occurrence)?;
                self.active.insert(position, ActiveBehaviour { occurrence, instance });
                if self.call_behaviours {
                    let mut context = self.context.copy_for_behaviour();
                    context.set_generated_instance(self.active[position].instance.as_ref().map(StateGraphContextHandle::shallow_copy));
                    backend.begin(&context, &ControllerState { current: self.current, last: self.last,
                        delta_time: dt, times: &self.times }, occurrence)?;
                }
            }
        }
        if self.call_behaviours {
            backend.generated_update(&mut self.context,
                &ControllerState { current: self.current, last: self.last, delta_time: dt, times: &self.times },
                declarations.states[usize::from(self.current)].update_address, &self.active)?;
        }
        Ok(())
    }
}

/// Entire EvaluateExpression<2>82925fb8. Native default is true, including
/// expressions that have only a precondition. All listed calls use the SAME
/// per-frame cache as EvaluateExpression<1>, not a separate sustain cache.
fn sustain(expression: u16, condition: &mut impl FnMut(u16) -> Result<bool, Error>) -> Result<bool, Error> {
    let index = match expression {
        38 => 40, 49 => 48, 51 => 53, 53 => 55, 55 => 57, 58 => 60, 60 => 62,
        72 => 76, 73 => 77, 74 => 78, 75 => 79, 76 => 80, 77 => 81, 78 => 82,
        79 => 83, 80 => 85, 82 => 88, 83 => 91, 84 => 93, 85 => 94, 93 => 99,
        _ => return Ok(true),
    };
    condition(index)
}
