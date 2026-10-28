//! Retained generated Action controller load, source constructor82927fc0.
//!
//! Native declarations and actual constructor fields are retained. Execution
//! callbacks are a separate boundary: unrecovered callbacks return their exact
//! source address. Host ownership/allocators do not claim Xenon ABI equivalence.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "generated Action load awaits production wrapper integration"
    )
)]
use crate::{
    motion_graph_entity_authority::{
        BoundMotionGraphContext, MotionGraphControllerToken, MotionGraphEntityAuthority,
        MotionGraphInstanceToken,
    },
    state_graph_context::StateGraphContextHandle,
};
use std::{
    collections::BTreeMap,
    fmt,
    sync::{Arc, Mutex},
};
#[path = "generated_action_conditions.rs"]
pub(crate) mod conditions;
#[path = "generated_action_load_construct.rs"]
pub(crate) mod construct;
#[path = "generated_action_load_data.rs"]
mod data;
#[path = "generated_action_ids.rs"]
pub(crate) mod ids;
#[path = "generated_action_runtime.rs"]
pub(crate) mod runtime;
#[cfg(test)]
#[path = "generated_action_load_tests.rs"]
mod tests;
pub(crate) use construct::{BehaviourObject, ConditionObject};

pub(crate) const NULL_INDEX: u16 = u16::MAX;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Section {
    pub(crate) count: u16,
    pub(crate) index: u16,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Attribute {
    pub(crate) key: &'static str,
    pub(crate) value: &'static str,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BehaviourDeclaration {
    pub(crate) name: &'static str,
    pub(crate) parent: u16,
    pub(crate) attributes: Section,
    pub(crate) parameters: Section,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ConditionDeclaration {
    pub(crate) name: &'static str,
    pub(crate) attributes: Section,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StateDeclaration {
    pub(crate) name: &'static str,
    pub(crate) interrupt_type: u8,
    pub(crate) active: bool,
    pub(crate) interrupt_target: u16,
    pub(crate) parent: u16,
    pub(crate) expression: u16,
    pub(crate) children: Section,
    pub(crate) behaviours: Section,
    pub(crate) transitions: Section,
    pub(crate) update_address: u32,
    pub(crate) trailing: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TransitionDeclaration {
    pub(crate) name: &'static str,
    pub(crate) priority: u8,
    pub(crate) target: u16,
    pub(crate) expression: u16,
    pub(crate) hooks: Section,
}
/// Source addresses are evidence labels, never executable host pointers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SourceBinding {
    pub(crate) name: &'static str,
    pub(crate) factory_helper: u32,
    pub(crate) allocation_size: u32,
    pub(crate) vtable: u32,
    // Behaviour: CreateInstance, Begin, Update, End.
    // Condition: GetActivationMasked, ActivationFunc, zero, zero.
    pub(crate) callbacks: [u32; 4],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ObjectDomain {
    Behaviour,
    Condition,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AllocationSite {
    pub(crate) domain: ObjectDomain,
    pub(crate) index: usize,
    pub(crate) binding: SourceBinding,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GeneratedActionAllocation {
    Host,
    Null,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum GeneratedActionLoadError {
    ContextRequired,
    ForeignContext,
    Lock,
    Allocation,
    UnregisteredClass(String),
    MissingAttribute(&'static str),
    InvalidAttribute(&'static str, String),
    DescriptorBounds,
    UnsupportedParameters,
    NullObject {
        domain: ObjectDomain,
        index: usize,
    },
    UnrecoveredCallback {
        domain: ObjectDomain,
        index: usize,
        address: u32,
    },
}
impl fmt::Display for GeneratedActionLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "generated Action load: {self:?}")
    }
}
impl std::error::Error for GeneratedActionLoadError {}

/// Selected concrete registrations in the actual Skater factory domain.
/// Construct empty, then register the recovered Skater classes at boot.
#[derive(Default)]
pub(crate) struct GeneratedActionFactories {
    behaviours: BTreeMap<&'static str, BehaviourClass>,
    conditions: BTreeMap<&'static str, ConditionClass>,
}
impl GeneratedActionFactories {
    pub(crate) fn new() -> Self {
        Self::default()
    }
    pub(crate) fn register_skater_classes(&mut self) -> Result<(), GeneratedActionLoadError> {
        for declaration in &data::BEHAVIOURS {
            let class = BehaviourClass::from_name(declaration.name)?;
            self.behaviours.entry(declaration.name).or_insert(class);
        }
        for declaration in &data::CONDITIONS {
            let class = ConditionClass::from_name(declaration.name)?;
            self.conditions.entry(declaration.name).or_insert(class);
        }
        Ok(())
    }
}
struct StaticState {
    printed: bool,
    behaviours_initialized: bool,
    conditions_initialized: bool,
    behaviours: [Option<Arc<BehaviourObject>>; 311],
    conditions: [Option<Arc<ConditionObject>>; 146],
    names: BTreeMap<&'static str, Arc<str>>,
}
/// Same shared native static domain across every loaded Action controller.
#[derive(Clone)]
pub(crate) struct GeneratedActionStatics {
    inner: Arc<Mutex<StaticState>>,
}
impl GeneratedActionStatics {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(StaticState {
                printed: false,
                behaviours_initialized: false,
                conditions_initialized: false,
                behaviours: std::array::from_fn(|_| None),
                conditions: std::array::from_fn(|_| None),
                names: BTreeMap::new(),
            })),
        }
    }
    pub(crate) fn behaviour(
        &self,
        index: usize,
    ) -> Result<Option<Arc<BehaviourObject>>, GeneratedActionLoadError> {
        self.inner
            .lock()
            .map_err(|_| GeneratedActionLoadError::Lock)?
            .behaviours
            .get(index)
            .cloned()
            .ok_or(GeneratedActionLoadError::DescriptorBounds)
    }
    pub(crate) fn condition(
        &self,
        index: usize,
    ) -> Result<Option<Arc<ConditionObject>>, GeneratedActionLoadError> {
        self.inner
            .lock()
            .map_err(|_| GeneratedActionLoadError::Lock)?
            .conditions
            .get(index)
            .cloned()
            .ok_or(GeneratedActionLoadError::DescriptorBounds)
    }
    fn try_initialize(
        &self,
        factories: &GeneratedActionFactories,
        allocate: &mut impl FnMut(
            AllocationSite,
        )
            -> Result<GeneratedActionAllocation, GeneratedActionLoadError>,
    ) -> Result<(), GeneratedActionLoadError> {
        let mut state = self
            .inner
            .lock()
            .map_err(|_| GeneratedActionLoadError::Lock)?;
        state.printed = true;
        if !state.behaviours_initialized {
            for (index, declaration) in data::BEHAVIOURS.iter().enumerate() {
                let class = *factories.behaviours.get(declaration.name).ok_or_else(|| {
                    GeneratedActionLoadError::UnregisteredClass(declaration.name.to_owned())
                })?;
                let object = match allocate(AllocationSite {
                    domain: ObjectDomain::Behaviour,
                    index,
                    binding: class.binding(),
                })? {
                    GeneratedActionAllocation::Host => {
                        let name = Arc::clone(
                            state
                                .names
                                .entry(declaration.name)
                                .or_insert_with(|| Arc::from(declaration.name)),
                        );
                        Some(Arc::new(construct::behaviour(
                            class,
                            name,
                            declaration.attributes,
                        )?))
                    }
                    GeneratedActionAllocation::Null => None,
                };
                state.behaviours[index] = object;
                if declaration.parameters.count != 0 {
                    return Err(GeneratedActionLoadError::UnsupportedParameters);
                }
                // Source stores sInit INSIDE the loop, including null results.
                state.behaviours_initialized = true;
            }
        }
        if !state.conditions_initialized {
            for (index, declaration) in data::CONDITIONS.iter().enumerate() {
                let class = *factories.conditions.get(declaration.name).ok_or_else(|| {
                    GeneratedActionLoadError::UnregisteredClass(declaration.name.to_owned())
                })?;
                state.conditions[index] = match allocate(AllocationSite {
                    domain: ObjectDomain::Condition,
                    index,
                    binding: class.binding(),
                })? {
                    GeneratedActionAllocation::Host => {
                        let name = Arc::clone(
                            state
                                .names
                                .entry(declaration.name)
                                .or_insert_with(|| Arc::from(declaration.name)),
                        );
                        Some(Arc::new(construct::condition(
                            class,
                            name,
                            declaration.attributes,
                        )?))
                    }
                    GeneratedActionAllocation::Null => None,
                };
            }
            state.conditions_initialized = true;
        }
        Ok(())
    }
}

/// Source has 42 null instance pairs, one null hook, five inactive cache words,
/// empty fixed-map(8), empty IndexSet(42), current/last FFFF and positive-zero dt.
/// Instance values remain absent until a real `CreateInstance` callback succeeds.
/// Native BoardAdjust::Data is 0x10 bytes: mState+8, mLastAngle+c.
/// CreateInstance82cbaa40 leaves mLastAngle uninitialized until Begin82cbaa20.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum BoardAdjustStateError {
    UninitializedLastAngle,
    InvalidState(u32),
}

pub(crate) struct BoardAdjustInstance {
    token: MotionGraphInstanceToken,
    state: u32,
    last_angle: Option<f32>,
}
impl BoardAdjustInstance {
    pub(crate) fn new() -> Self {
        Self {
            token: MotionGraphInstanceToken::issue(),
            state: 0,
            last_angle: None,
        }
    }
    pub(crate) fn begin(&mut self) {
        self.state = 0;
        self.last_angle = Some(0.0);
    }
    pub(crate) fn reset_after_missing_intent(&mut self) {
        // 82cd5910 writes last angle, then 82cd5914 resets state.
        self.last_angle = Some(0.0);
        self.state = 0;
    }
    pub(crate) fn update_angle(&mut self, angle: f32) -> Result<f32, BoardAdjustStateError> {
        use std::f32::consts::{FRAC_PI_2, PI};
        let last = self
            .last_angle
            .ok_or(BoardAdjustStateError::UninitializedLastAngle)?;
        if self.state > 2 {
            return Err(BoardAdjustStateError::InvalidState(self.state));
        }
        // 82cd5790..57b8: ordered negative-product and |last| > half-pi.
        let reversed = last * angle < 0.0 && last.abs() > FRAC_PI_2;
        if reversed {
            match self.state {
                0 if last < 0.0 => self.state = 1,
                0 if last > 0.0 => self.state = 2,
                1 if last > 0.0 => self.state = 0,
                2 if last < 0.0 => self.state = 0,
                _ => {}
            }
        }
        self.last_angle = Some(angle); // 82cd5860 before output map write.
        Ok(match self.state {
            0 => angle,
            1 => -PI,
            2 => PI,
            _ => unreachable!("validated BoardAdjust state"),
        })
    }
    pub(crate) fn token(&self) -> &MotionGraphInstanceToken {
        &self.token
    }
}

pub(crate) enum GeneratedActionInstance {
    Gesture(crate::actor_construction::action_motion_intent_bridge::gesture::GestureTrickInstance),
    BodyFlip(
        crate::actor_construction::action_motion_intent_bridge::ground::BodyFlippingSignalInstance,
    ),
    TimeIntent(crate::actor_construction::action_motion_intent_bridge::ground::TimeIntentInstance),
    ConstIntent(
        crate::actor_construction::action_motion_intent_bridge::constant::ConstIntentInstance,
    ),
    TransferIntent(crate::actor_construction::action_motion_intent_bridge::TransferIntentInstance),
    JuiceHook(crate::actor_construction::action_motion_intent_bridge::JuiceHookInstance),
    BoardAdjust(BoardAdjustInstance),
}
pub(crate) enum GeneratedActionHook {}
pub(crate) struct BehaviourInstanceSlot {
    pub(crate) behaviour: Option<Arc<BehaviourObject>>,
    pub(crate) instance: Option<GeneratedActionInstance>,
}
pub(crate) struct GeneratedActionController<K> {
    entity: MotionGraphEntityAuthority<K>,
    wrapper: MotionGraphControllerToken,
    statics: GeneratedActionStatics,
    states: &'static [StateDeclaration; 224],
    transitions: &'static [TransitionDeclaration; 129],
    transition_indices: &'static [u16; 129],
    pub(crate) current_state: u16,
    pub(crate) last_state: u16,
    pub(crate) delta_time: f32,
    pub(crate) time_in_state: BTreeMap<u16, f32>,
    pub(crate) current_behaviours: Vec<u16>,
    pub(crate) instance_slots: [BehaviourInstanceSlot; 42],
    pub(crate) hook: Option<GeneratedActionHook>,
    pub(crate) condition_active: [u32; 5],
    // Native result bits are uninitialized until their active bit is set.
    pub(crate) condition_values: [Option<bool>; 146],
    pub(crate) call_behaviours: bool,
    pub(crate) allow_evaluations: bool,
}
impl<K> GeneratedActionController<K> {
    /// Selected host allocation. Debug arguments of the release abstract
    /// constructor82cef238 are unused and do not create stored tuning.
    pub(crate) fn try_load(
        context: &BoundMotionGraphContext<K>,
        statics: &GeneratedActionStatics,
        factories: &GeneratedActionFactories,
        call_behaviours: bool,
    ) -> Result<Self, GeneratedActionLoadError> {
        Self::try_load_with_allocations(context, statics, factories, call_behaviours, &mut |_| {
            Ok(GeneratedActionAllocation::Host)
        })
    }
    pub(crate) fn try_load_with_allocations(
        context: &BoundMotionGraphContext<K>,
        statics: &GeneratedActionStatics,
        factories: &GeneratedActionFactories,
        call_behaviours: bool,
        allocate: &mut impl FnMut(
            AllocationSite,
        )
            -> Result<GeneratedActionAllocation, GeneratedActionLoadError>,
    ) -> Result<Self, GeneratedActionLoadError> {
        let entity = context
            .entity()
            .ok_or(GeneratedActionLoadError::ContextRequired)?
            .shallow_copy();
        let wrapper = context
            .controller()
            .ok_or(GeneratedActionLoadError::ContextRequired)?
            .shallow_copy();
        let mut current_behaviours = Vec::new();
        current_behaviours
            .try_reserve_exact(42)
            .map_err(|_| GeneratedActionLoadError::Allocation)?;
        let result = Self {
            entity,
            wrapper,
            statics: statics.clone(),
            current_state: NULL_INDEX,
            last_state: NULL_INDEX,
            states: &data::STATES,
            transitions: &data::TRANSITIONS,
            transition_indices: &data::INDICES,
            delta_time: 0.0,
            time_in_state: BTreeMap::new(),
            current_behaviours,
            instance_slots: std::array::from_fn(|_| BehaviourInstanceSlot {
                behaviour: None,
                instance: None,
            }),
            hook: None,
            condition_active: [0; 5],
            condition_values: [None; 146],
            call_behaviours,
            allow_evaluations: false,
        };
        statics.try_initialize(factories, allocate)?;
        Ok(result)
    }
    pub(crate) fn validate_context(
        &self,
        context: &BoundMotionGraphContext<K>,
    ) -> Result<(), GeneratedActionLoadError> {
        if context.entity() != Some(&self.entity) || context.controller() != Some(&self.wrapper) {
            return Err(GeneratedActionLoadError::ForeignContext);
        }
        Ok(())
    }
    pub(crate) const fn root_state(&self) -> &'static StateDeclaration {
        &self.states[0]
    }
    pub(crate) const fn states(&self) -> &'static [StateDeclaration; 224] {
        self.states
    }
    pub(crate) const fn transitions(&self) -> &'static [TransitionDeclaration; 129] {
        self.transitions
    }
    pub(crate) const fn transition_indices(&self) -> &'static [u16; 129] {
        self.transition_indices
    }
    /// Exact virtual `ActivationFunc` boundary, deliberately no successful
    /// substitute for a physical, animation, or intent collaborator.
    pub(crate) fn try_condition_activation(
        &self,
        context: &BoundMotionGraphContext<K>,
        index: usize,
    ) -> Result<bool, GeneratedActionLoadError> {
        self.validate_context(context)?;
        let object =
            self.statics
                .condition(index)?
                .ok_or(GeneratedActionLoadError::NullObject {
                    domain: ObjectDomain::Condition,
                    index,
                })?;
        Err(GeneratedActionLoadError::UnrecoveredCallback {
            domain: ObjectDomain::Condition,
            index,
            address: object.class.binding().callbacks[1],
        })
    }
    /// First behaviour-enter operation, virtual `CreateInstance`(+2c).
    pub(crate) fn try_create_behaviour_instance(
        &self,
        context: &BoundMotionGraphContext<K>,
        index: usize,
    ) -> Result<(), GeneratedActionLoadError> {
        self.validate_context(context)?;
        let object =
            self.statics
                .behaviour(index)?
                .ok_or(GeneratedActionLoadError::NullObject {
                    domain: ObjectDomain::Behaviour,
                    index,
                })?;
        Err(GeneratedActionLoadError::UnrecoveredCallback {
            domain: ObjectDomain::Behaviour,
            index,
            address: object.class.binding().callbacks[0],
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BehaviourClass {
    JuiceHook,
    CreateMGIntentFromAGIntent,
    PrintText2D,
    CreateConstMGIntent,
    BodyFlippingSignal,
    CreateMGTimeIntentFromAGIntent,
    CreateTrickIntentFromGesture,
    BoardAdjust,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConditionClass {
    HasAGIntent,
    PhysFilteredState,
    TimeSinceLastInput,
    PhysicsSpeedCompare,
    InParentStateForTime,
    IsGrinding,
    IsMirrored,
    IsHandPlanting,
    IsFootPlanting,
    CurrentState,
    TimeToLand,
    DisablePushBrake,
    PhysicsSpeedAndSlopeCompare,
    IsRidingFakie,
    HasGestureIntent,
    PhysicsRequestsDismount,
    IsGrabbingObject,
    IsDismounting,
    HasAnimAttribute,
    IsInLocomotion,
    IsLandingOnBoard,
    OBTimeToLand,
    PhysicsWantsRunout,
    IsDoneAirDismount,
}
impl BehaviourClass {
    pub(super) fn from_name(name: &str) -> Result<Self, GeneratedActionLoadError> {
        match name {
            "JuiceHook" => Ok(Self::JuiceHook),
            "CreateMGIntentFromAGIntent" => Ok(Self::CreateMGIntentFromAGIntent),
            "PrintText2D" => Ok(Self::PrintText2D),
            "CreateConstMGIntent" => Ok(Self::CreateConstMGIntent),
            "BodyFlippingSignal" => Ok(Self::BodyFlippingSignal),
            "CreateMGTimeIntentFromAGIntent" => Ok(Self::CreateMGTimeIntentFromAGIntent),
            "CreateTrickIntentFromGesture" => Ok(Self::CreateTrickIntentFromGesture),
            "BoardAdjust" => Ok(Self::BoardAdjust),
            _ => Err(GeneratedActionLoadError::UnregisteredClass(name.to_owned())),
        }
    }
    pub(crate) const fn binding(self) -> SourceBinding {
        match self {
            Self::JuiceHook => SourceBinding {
                name: "JuiceHook",
                factory_helper: 0x82c8_f2a8,
                allocation_size: 0x1c,
                vtable: 0x822f_ef50,
                callbacks: [0x82cb_ab70, 0x8251_87c0, 0x82cd_4290, 0x8251_87c0],
            },
            Self::CreateMGIntentFromAGIntent => SourceBinding {
                name: "CreateMGIntentFromAGIntent",
                factory_helper: 0x82cb_ef68,
                allocation_size: 0x68,
                vtable: 0x8230_2ea8,
                callbacks: [0x82cb_a730, 0x82cd_6290, 0x82cd_62c8, 0x82cd_4288],
            },
            Self::PrintText2D => SourceBinding {
                name: "PrintText2D",
                factory_helper: 0x82c8_fc08,
                allocation_size: 0x428,
                vtable: 0x8230_0930,
                callbacks: [0x82bb_3da8, 0x8251_87c0, 0x8251_87c0, 0x8251_87c0],
            },
            Self::CreateConstMGIntent => SourceBinding {
                name: "CreateConstMGIntent",
                factory_helper: 0x82cb_6138,
                allocation_size: 0x3c,
                vtable: 0x8230_1ed8,
                callbacks: [0x8271_26f8, 0x82cd_6258, 0x82cd_4230, 0x82cd_4280],
            },
            Self::BodyFlippingSignal => SourceBinding {
                name: "BodyFlippingSignal",
                factory_helper: 0x82c8_f248,
                allocation_size: 0x1c,
                vtable: 0x822f_ef14,
                callbacks: [0x82ca_ec70, 0x82c9_4840, 0x82cd_5cd8, 0x8251_87c0],
            },
            Self::CreateMGTimeIntentFromAGIntent => SourceBinding {
                name: "CreateMGTimeIntentFromAGIntent",
                factory_helper: 0x82cb_6198,
                allocation_size: 0x4c,
                vtable: 0x8230_1f20,
                callbacks: [0x8271_2148, 0x8251_87c0, 0x82cd_5928, 0x82cc_2028],
            },
            Self::CreateTrickIntentFromGesture => SourceBinding {
                name: "CreateTrickIntentFromGesture",
                factory_helper: 0x82cb_efc8,
                allocation_size: 0x3c,
                vtable: 0x8230_2e18,
                callbacks: [0x82cb_a2b8, 0x82cd_4da8, 0x82cd_51b0, 0x82cc_1d58],
            },
            Self::BoardAdjust => SourceBinding {
                name: "BoardAdjust",
                factory_helper: 0x82cb_f028,
                allocation_size: 0x54,
                vtable: 0x8230_2f1c,
                callbacks: [0x82cb_aa40, 0x82cb_aa20, 0x82cd_54c0, 0x82cc_1f38],
            },
        }
    }
}
impl ConditionClass {
    pub(super) fn from_name(name: &str) -> Result<Self, GeneratedActionLoadError> {
        match name {
            "HasAGIntent" => Ok(Self::HasAGIntent),
            "PhysFilteredState" => Ok(Self::PhysFilteredState),
            "TimeSinceLastInput" => Ok(Self::TimeSinceLastInput),
            "PhysicsSpeedCompare" => Ok(Self::PhysicsSpeedCompare),
            "InParentStateForTime" => Ok(Self::InParentStateForTime),
            "IsGrinding" => Ok(Self::IsGrinding),
            "IsMirrored" => Ok(Self::IsMirrored),
            "IsHandPlanting" => Ok(Self::IsHandPlanting),
            "IsFootPlanting" => Ok(Self::IsFootPlanting),
            "CurrentState" => Ok(Self::CurrentState),
            "TimeToLand" => Ok(Self::TimeToLand),
            "DisablePushBrake" => Ok(Self::DisablePushBrake),
            "PhysicsSpeedAndSlopeCompare" => Ok(Self::PhysicsSpeedAndSlopeCompare),
            "IsRidingFakie" => Ok(Self::IsRidingFakie),
            "HasGestureIntent" => Ok(Self::HasGestureIntent),
            "PhysicsRequestsDismount" => Ok(Self::PhysicsRequestsDismount),
            "IsGrabbingObject" => Ok(Self::IsGrabbingObject),
            "IsDismounting" => Ok(Self::IsDismounting),
            "HasAnimAttribute" => Ok(Self::HasAnimAttribute),
            "IsInLocomotion" => Ok(Self::IsInLocomotion),
            "IsLandingOnBoard" => Ok(Self::IsLandingOnBoard),
            "OBTimeToLand" => Ok(Self::OBTimeToLand),
            "PhysicsWantsRunout" => Ok(Self::PhysicsWantsRunout),
            "IsDoneAirDismount" => Ok(Self::IsDoneAirDismount),
            _ => Err(GeneratedActionLoadError::UnregisteredClass(name.to_owned())),
        }
    }
    #[expect(
        clippy::too_many_lines,
        reason = "one declarative table retains all 24 source condition bindings for independent auditing"
    )]
    pub(crate) const fn binding(self) -> SourceBinding {
        match self {
            Self::HasAGIntent => SourceBinding {
                name: "HasAGIntent",
                factory_helper: 0x82cb_6018,
                allocation_size: 0x40,
                vtable: 0x8230_1df4,
                callbacks: [0x82ce_f0f8, 0x82cc_1748, 0x0, 0x0],
            },
            Self::PhysFilteredState => SourceBinding {
                name: "PhysFilteredState",
                factory_helper: 0x82c8_f368,
                allocation_size: 0x20,
                vtable: 0x822f_f05c,
                callbacks: [0x82ce_f0f8, 0x82c9_9618, 0x0, 0x0],
            },
            Self::TimeSinceLastInput => SourceBinding {
                name: "TimeSinceLastInput",
                factory_helper: 0x82c8_f068,
                allocation_size: 0x28,
                vtable: 0x822f_ee04,
                callbacks: [0x82ce_f0f8, 0x82c9_8b78, 0x0, 0x0],
            },
            Self::PhysicsSpeedCompare => SourceBinding {
                name: "PhysicsSpeedCompare",
                factory_helper: 0x82c8_f6c8,
                allocation_size: 0x2c,
                vtable: 0x822f_f0d4,
                callbacks: [0x82ce_f0f8, 0x82c9_9808, 0x0, 0x0],
            },
            Self::InParentStateForTime => SourceBinding {
                name: "InParentStateForTime",
                factory_helper: 0x82c8_f788,
                allocation_size: 0x28,
                vtable: 0x822f_f21c,
                callbacks: [0x82ce_f0f8, 0x82c8_c950, 0x0, 0x0],
            },
            Self::IsGrinding => SourceBinding {
                name: "IsGrinding",
                factory_helper: 0x82cb_61f8,
                allocation_size: 0x34,
                vtable: 0x8230_1f68,
                callbacks: [0x82ce_f0f8, 0x82ca_a310, 0x0, 0x0],
            },
            Self::IsMirrored => SourceBinding {
                name: "IsMirrored",
                factory_helper: 0x82c8_f848,
                allocation_size: 0x1c,
                vtable: 0x822f_f288,
                callbacks: [0x82ce_f0f8, 0x82c9_a210, 0x0, 0x0],
            },
            Self::IsHandPlanting => SourceBinding {
                name: "IsHandPlanting",
                factory_helper: 0x82c8_f9c8,
                allocation_size: 0x24,
                vtable: 0x822f_f448,
                callbacks: [0x82ce_f0f8, 0x82c9_abf0, 0x0, 0x0],
            },
            Self::IsFootPlanting => SourceBinding {
                name: "IsFootPlanting",
                factory_helper: 0x82c9_2428,
                allocation_size: 0x1c,
                vtable: 0x8230_1140,
                callbacks: [0x82ce_f0f8, 0x82ca_7830, 0x0, 0x0],
            },
            Self::CurrentState => SourceBinding {
                name: "CurrentState",
                factory_helper: 0x82ca_c3c0,
                allocation_size: 0x2c,
                vtable: 0x8230_1be8,
                callbacks: [0x82ce_f0f8, 0x82c9_4ae0, 0x0, 0x0],
            },
            Self::TimeToLand => SourceBinding {
                name: "TimeToLand",
                factory_helper: 0x82c9_0508,
                allocation_size: 0x28,
                vtable: 0x822f_fa54,
                callbacks: [0x82ce_f0f8, 0x82c9_cc40, 0x0, 0x0],
            },
            Self::DisablePushBrake => SourceBinding {
                name: "DisablePushBrake",
                factory_helper: 0x82c8_f608,
                allocation_size: 0x1c,
                vtable: 0x822f_f3d4,
                callbacks: [0x82ce_f0f8, 0x82c9_a8f0, 0x0, 0x0],
            },
            Self::PhysicsSpeedAndSlopeCompare => SourceBinding {
                name: "PhysicsSpeedAndSlopeCompare",
                factory_helper: 0x82c8_f668,
                allocation_size: 0x28,
                vtable: 0x822f_f090,
                callbacks: [0x82ce_f0f8, 0x82c9_9710, 0x0, 0x0],
            },
            Self::IsRidingFakie => SourceBinding {
                name: "IsRidingFakie",
                factory_helper: 0x82c8_f8a8,
                allocation_size: 0x1c,
                vtable: 0x822f_f2bc,
                callbacks: [0x82ce_f0f8, 0x82c9_a308, 0x0, 0x0],
            },
            Self::HasGestureIntent => SourceBinding {
                name: "HasGestureIntent",
                factory_helper: 0x82c8_f008,
                allocation_size: 0x20,
                vtable: 0x822f_edd0,
                callbacks: [0x82ce_f0f8, 0x82cc_1870, 0x0, 0x0],
            },
            Self::PhysicsRequestsDismount => SourceBinding {
                name: "PhysicsRequestsDismount",
                factory_helper: 0x82c8_f0c8,
                allocation_size: 0x1c,
                vtable: 0x822f_ee3c,
                callbacks: [0x82ce_f0f8, 0x82c9_8c70, 0x0, 0x0],
            },
            Self::IsGrabbingObject => SourceBinding {
                name: "IsGrabbingObject",
                factory_helper: 0x82c8_fb48,
                allocation_size: 0x1c,
                vtable: 0x822f_f4e8,
                callbacks: [0x82ce_f0f8, 0x82c9_aef0, 0x0, 0x0],
            },
            Self::IsDismounting => SourceBinding {
                name: "IsDismounting",
                factory_helper: 0x82c8_fae8,
                allocation_size: 0x28,
                vtable: 0x822f_f4b0,
                callbacks: [0x82ce_f0f8, 0x82c9_ad78, 0x0, 0x0],
            },
            Self::HasAnimAttribute => SourceBinding {
                name: "HasAnimAttribute",
                factory_helper: 0x82ca_c2a0,
                allocation_size: 0x44,
                vtable: 0x8230_1b44,
                callbacks: [0x82ce_f0f8, 0x82ca_a1d0, 0x0, 0x0],
            },
            Self::IsInLocomotion => SourceBinding {
                name: "IsInLocomotion",
                factory_helper: 0x82c8_f188,
                allocation_size: 0x1c,
                vtable: 0x822f_eea4,
                callbacks: [0x82ce_f0f8, 0x82c9_8e78, 0x0, 0x0],
            },
            Self::IsLandingOnBoard => SourceBinding {
                name: "IsLandingOnBoard",
                factory_helper: 0x82c8_f128,
                allocation_size: 0x1c,
                vtable: 0x822f_ee70,
                callbacks: [0x82ce_f0f8, 0x82c9_8d70, 0x0, 0x0],
            },
            Self::OBTimeToLand => SourceBinding {
                name: "OBTimeToLand",
                factory_helper: 0x82c8_fba8,
                allocation_size: 0x28,
                vtable: 0x822f_f51c,
                callbacks: [0x82ce_f0f8, 0x82c9_aff0, 0x0, 0x0],
            },
            Self::PhysicsWantsRunout => SourceBinding {
                name: "PhysicsWantsRunout",
                factory_helper: 0x82c8_f488,
                allocation_size: 0x1c,
                vtable: 0x822f_f1e8,
                callbacks: [0x82ce_f0f8, 0x82c9_9f98, 0x0, 0x0],
            },
            Self::IsDoneAirDismount => SourceBinding {
                name: "IsDoneAirDismount",
                factory_helper: 0x82c9_0808,
                allocation_size: 0x28,
                vtable: 0x822f_fc64,
                callbacks: [0x82ce_f0f8, 0x82ca_efe0, 0x0, 0x0],
            },
        }
    }
}
