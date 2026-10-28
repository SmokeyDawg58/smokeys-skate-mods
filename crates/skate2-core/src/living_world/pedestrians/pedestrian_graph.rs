//! Generated pedestrian TemplateHC lifecycle82956488. Leaf callbacks remain
//! concrete backend operations; this owner never invents successful behaviors.
use crate::{
    generated_action_load::{NULL_INDEX, runtime::ActionUpdateError as Error},
    generated_graph_selection::{Graph, Mask},
    pedestrian_graph_data as data,
};
use std::collections::BTreeMap;
#[path = "pedestrian_graph_expressions.rs"]
pub(crate) mod expressions;
#[path = "pedestrian_graph_ids.rs"]
pub(crate) mod ids;
#[path = "pedestrian_graph_conditions.rs"]
pub(crate) mod conditions;
#[path = "pedestrian_graph_condition_binding.rs"]
pub(crate) mod condition_binding;
#[path = "pedestrian_graph_locomotion.rs"]
pub(crate) mod locomotion;
#[path = "pedestrian_graph_simple_behaviours.rs"]
pub(crate) mod simple_behaviours;
#[path = "pedestrian_graph_updates.rs"]
pub(crate) mod updates;
#[path = "pedestrian_graph_lifecycle.rs"]
pub(crate) mod lifecycle;
#[path = "pedestrian_graph_colliding.rs"]
pub(crate) mod colliding;
#[path = "pedestrian_graph_rest.rs"]
pub(crate) mod rest;
#[path = "pedestrian_graph_behaviour_binding.rs"]
pub(crate) mod behaviour_binding;
#[path = "pedestrian_graph_backend.rs"]
pub(crate) mod backend;
#[path = "pedestrian_marquee_graph_data.rs"]
pub(crate) mod marquee_data;
#[path = "pedestrian_graph_marquee.rs"]
pub(crate) mod marquee;
#[path = "pedestrian_big_black_graph_data.rs"]
pub(crate) mod big_black_data;
#[path = "pedestrian_big_black_graph_ids.rs"]
pub(crate) mod big_black_ids;
#[path = "pedestrian_graph_big_black.rs"]
pub(crate) mod big_black;

/// The generated AI delegates a pedestrian controller runs:
/// `GENERATED_STATE_GRAPH_DELEGATE_BASENAMES` 5 (`Pedestrian.xml`), 16 (`marquee.xml`) and
/// 19 (`Photographer.xml`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AiDelegate {
    Pedestrian,
    BigBlack,
    Marquee,
    Photographer,
}

impl AiDelegate {
    pub(crate) const fn from_delegate_index(index: usize) -> Option<Self> {
        match index {
            3 => Some(Self::BigBlack),
            5 => Some(Self::Pedestrian),
            16 => Some(Self::Marquee),
            19 => Some(Self::Photographer),
            _ => None,
        }
    }

    /// `kStates`: HCDelegate_Pedestrian 0x821d25d0, HCDelegate_marquee 0x821da2f0 and
    /// HCDelegate_Photographer 0x821db730.
    pub(crate) fn states(self) -> &'static [crate::generated_action_load::StateDeclaration] {
        match self {
            Self::Pedestrian => &data::STATES,
            Self::BigBlack => &big_black_data::big_black::STATES,
            Self::Marquee => &marquee_data::marquee::STATES,
            Self::Photographer => &marquee_data::photographer::STATES,
        }
    }
}

/// Loaded generated AI declarations, distinct from active occurrence payloads
/// and graph state. An unsupported callback remains a dispatch error.
pub(crate) struct PedestrianAiBindings {
    pub(crate) delegate: AiDelegate,
    pub(crate) path: Vec<u8>,
    pub(crate) root_name_hash: u32,
    pub(crate) conditions: std::sync::Arc<condition_binding::PedestrianConditions>,
    pub(crate) behaviours: behaviour_binding::LocalBehaviourBindings,
}

pub(crate) struct PedestrianGraph {
    pub(crate) current: u16,
    pub(crate) last: u16,
    pub(crate) delta_time: f32,
    times: BTreeMap<u16, f32>,
}
impl Default for PedestrianGraph {
    fn default() -> Self {
        Self { current: NULL_INDEX, last: NULL_INDEX, delta_time: 0., times: BTreeMap::new() }
    }
}
/// The backend retains the actual occurrence instances and copies native
/// Context's handle slots when dispatching Begin/End, as the shared controller
/// does. Generated Update gets the ordered instance set, not virtual updates.
pub(crate) trait PedestrianGraphBackend {
    fn condition(&mut self, graph: &PedestrianGraph, mask: Mask, condition: u16,
        state: u16) -> Result<bool, Error>;
    fn enabled(&self, occurrence: u16) -> Result<bool, Error>;
    fn create_instance(&mut self, graph: &PedestrianGraph, occurrence: u16) -> Result<(), Error>;
    fn begin(&mut self, graph: &PedestrianGraph, occurrence: u16) -> Result<(), Error>;
    fn end_and_remove(&mut self, graph: &PedestrianGraph, occurrence: u16) -> Result<(), Error>;
    fn transition_hook(&mut self, graph: &PedestrianGraph, hook: u16,
        previous: u16) -> Result<(), Error>;
    fn generated_update(&mut self, graph: &PedestrianGraph, source: u32,
        active: &[u16]) -> Result<(), Error>;
}
/// Native sorted IndexSet<20>; no second entity or behavior roster.
#[derive(Default)]
pub(crate) struct PedestrianGraphBehaviours { active: Vec<u16> }

/// Declarations for the same generated TemplateHC runtime. Family-specific
/// predicates remain supplied by that generated delegate.
pub(crate) struct GeneratedDeclarations {
    pub(crate) states: &'static [crate::generated_action_load::StateDeclaration],
    pub(crate) transitions: &'static [crate::generated_action_load::TransitionDeclaration],
    pub(crate) indices: &'static [u16],
    pub(crate) behaviours: &'static [crate::generated_action_load::BehaviourDeclaration],
    pub(crate) active_capacity: usize,
}
impl PedestrianGraph {
    /// TemplateHC.EndAllBehaviours8293df00: descending occurrence removal;
    /// state identity and timers are retained by this operation.
    pub(crate) fn end_all(&self,instances:&mut PedestrianGraphBehaviours,
        backend:&mut impl PedestrianGraphBackend)->Result<(),Error>{
        while let Some(&occurrence)=instances.active.last(){
            backend.end_and_remove(self,occurrence)?;
            instances.active.pop();
        }
        Ok(())
    }
    pub(crate) fn time_in_state(&self, state: u16) -> f32 {
        self.times.get(&state).copied().unwrap_or(-1.)
    }
    /// The delegate's `TemplateHC::Update`: HCDelegate_marquee 0x829575b8 and
    /// HCDelegate_Photographer 0x82957a68 run `IndexSet<3>` hierarchies with four and six
    /// behaviour instances.
    pub(crate) fn update_delegate(&mut self, delegate: AiDelegate, dt: f32,
        instances: &mut PedestrianGraphBehaviours, backend: &mut impl PedestrianGraphBackend,
    ) -> Result<(), Error> {
        use marquee_data::{marquee, photographer};
        match delegate {
            AiDelegate::Pedestrian => self.update(dt, instances, backend),
            AiDelegate::BigBlack => self.update_declarations::<7>(dt, instances, backend,
                &GeneratedDeclarations { states: &big_black_data::big_black::STATES,
                    transitions: &big_black_data::big_black::TRANSITIONS,
                    indices: &big_black_data::big_black::INDICES,
                    behaviours: &big_black_data::big_black::BEHAVIOURS, active_capacity: 16 },
                |mask, expression, c| big_black::evaluate(mask, expression, c)),
            AiDelegate::Marquee => self.update_declarations::<3>(dt, instances, backend,
                &GeneratedDeclarations { states: &marquee::STATES, transitions: &marquee::TRANSITIONS,
                    indices: &marquee::INDICES, behaviours: &marquee::BEHAVIOURS, active_capacity: 4 },
                |mask, expression, c| expressions::evaluate_marquee(mask, expression, c)),
            AiDelegate::Photographer => self.update_declarations::<3>(dt, instances, backend,
                &GeneratedDeclarations { states: &photographer::STATES,
                    transitions: &photographer::TRANSITIONS, indices: &photographer::INDICES,
                    behaviours: &photographer::BEHAVIOURS, active_capacity: 6 },
                |mask, expression, c| expressions::evaluate_photographer(mask, expression, c)),
        }
    }
    pub(crate) fn update(&mut self, dt: f32, instances: &mut PedestrianGraphBehaviours,
        backend: &mut impl PedestrianGraphBackend) -> Result<(), Error>
    {
        self.update_declarations::<9>(dt,instances,backend,&GeneratedDeclarations {
            states:&data::STATES,transitions:&data::TRANSITIONS,indices:&data::INDICES,
            behaviours:&data::BEHAVIOURS,active_capacity:20,
        }, |mask,expression,c|expressions::evaluate(mask,expression,c))
    }
    pub(crate) fn update_declarations<const DEPTH:usize>(&mut self, dt:f32,
        instances:&mut PedestrianGraphBehaviours, backend:&mut impl PedestrianGraphBackend,
        declarations:&GeneratedDeclarations,
        mut evaluate:impl FnMut(Mask,u16,&mut dyn FnMut(u16)->Result<bool,Error>)->Result<bool,Error>,
    )->Result<(),Error>{
        self.delta_time = dt;
        let graph = Graph::<DEPTH> { states: declarations.states, transitions: declarations.transitions,
            indices: declarations.indices, current: self.current };
        let selected = graph.next(&mut |mask, expression, state|
            evaluate(mask, expression, &mut |condition|
                backend.condition(self, mask, condition, state)))?;
        if selected.state == NULL_INDEX { return Err(Error::NoSelectedState); }
        let (exited, entered) = graph.changes(&selected)?;
        for time in self.times.values_mut() { *time += dt; }
        self.last = self.current;
        self.current = selected.state;
        // UpdateExitStates82934950: descending occurrence order, then timers.
        for position in (0..instances.active.len()).rev() {
            let occurrence = instances.active[position];
            let declaration = &declarations.behaviours[usize::from(occurrence)];
            if declaration.parent != NULL_INDEX && exited.contains(&declaration.parent) {
                backend.end_and_remove(self, occurrence)?;
                instances.active.remove(position);
            }
        }
        for state in exited {
            if self.times.remove(&state).is_none() { return Err(Error::InvalidActiveSet); }
        }
        if selected.transition != NULL_INDEX {
            // UpdateTransition82933c30 uses hook indices, and calls nothing
            // for an ordinary transition with an empty hook section.
            let hooks = declarations.transitions[usize::from(selected.transition)].hooks;
            for position in hooks.index..hooks.index.saturating_add(hooks.count) {
                let hook = *declarations.indices.get(usize::from(position))
                    .ok_or(Error::InvalidActiveSet)?;
                backend.transition_hook(self, hook, self.last)?;
            }
        }
        // UpdateEnterStates82953518: numeric state order, then occurrence order.
        for state in entered {
            if !self.times.contains_key(&state) && self.times.len() == DEPTH { return Err(Error::Capacity); }
            self.times.insert(state, 0.);
            let section = declarations.states[usize::from(state)].behaviours;
            for occurrence in section.index..section.index + section.count {
                if !backend.enabled(occurrence)? { continue; }
                let position = instances.active.binary_search(&occurrence)
                    .map_or_else(|position| Ok(position), |_| Err(Error::InvalidActiveSet))?;
                if instances.active.len() == declarations.active_capacity { return Err(Error::Capacity); }
                backend.create_instance(self, occurrence)?;
                // Native publishes the occurrence before calling Begin.
                instances.active.insert(position, occurrence);
                backend.begin(self, occurrence)?;
            }
        }
        backend.generated_update(self, declarations.states[usize::from(self.current)].update_address,
            &instances.active)
    }
}
