//! Evidence-bounded construction and structural finalization for `StateGraph`.
//!
//! This module preserves the matched debug build's direct object construction,
//! parent attachment, parser-stack, `Find`, and `Process` ordering. Concrete
//! condition, behaviour, and transition-hook semantics remain opaque factory
//! callbacks; a successfully built graph is therefore not gameplay-ready.
//! Arena enumeration retains logically unreachable replaced expressions and
//! parentless objects created beneath null stack slots as diagnostics. Owning
//! IDs and `Process` traversal preserve the live graph; native intrusive
//! release/destructor timing is deliberately not modeled.

use skate2_formats::state_graph::{
    STATE_GRAPH_BINARY_ATTRIBUTE_CAPACITY, STATE_GRAPH_PARSER_STACK_CAPACITY,
    StateGraphBinaryAttribute, StateGraphBinaryElement, state_graph_string_hash32,
};
use std::{error::Error, fmt};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct StateId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TransitionId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ExpressionId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LeafId(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BaseId {
    State(StateId),
    Transition(TransitionId),
    Expression(ExpressionId),
    Leaf(LeafId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LeafKind {
    Condition,
    Behaviour,
    Hook,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FactoryMissPolicy {
    /// Mirror the factory's non-safe assertion as a checked host error.
    Strict,
    /// Mirror native factory safe mode: return null and push a null stack slot.
    NativeSafeNull,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BaseFields {
    pub name: Vec<u8>,
    pub path: Vec<u8>,
    pub parent: Option<BaseId>,
    pub enabled: bool,
    pub cast_parent: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Interruptable {
    NotInterruptable,
    RootInterruptable,
    StateInterruptable {
        target_name: Vec<u8>,
        target: Option<StateId>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum TransitionPriority {
    Low = 0,
    Medium = 1,
    High = 2,
    Urgent = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum ExpressionOperator {
    None = 0,
    And = 1,
    Or = 2,
    Not = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum ConditionMask {
    PreCond = 1,
    Sustain = 2,
    Always = 3,
    PostCond = 4,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpressionChildId {
    Expression(ExpressionId),
    Condition(LeafId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateNode {
    pub base: BaseFields,
    pub parent_state: Option<StateId>,
    pub expression: Option<ExpressionId>,
    pub active: bool,
    pub interruptable: Interruptable,
    pub children: Vec<StateId>,
    pub behaviours: Vec<LeafId>,
    pub transitions: Vec<TransitionId>,
    pub depth: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransitionNode {
    pub base: BaseFields,
    pub parent_state: StateId,
    pub expression: Option<ExpressionId>,
    pub target_name: Vec<u8>,
    pub target: Option<StateId>,
    pub priority: TransitionPriority,
    pub hooks: Vec<LeafId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpressionNode {
    pub base: BaseFields,
    pub operator: ExpressionOperator,
    pub children: Vec<ExpressionChildId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LeafNode<P> {
    pub base: BaseFields,
    pub kind: LeafKind,
    pub registered_name: Vec<u8>,
    pub mask: Option<ConditionMask>,
    payload: Option<P>,
}

impl<P> LeafNode<P> {
    #[must_use]
    pub const fn payload(&self) -> Option<&P> {
        self.payload.as_ref()
    }

    #[must_use]
    pub fn payload_mut(&mut self) -> Option<&mut P> {
        self.payload.as_mut()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParamRecord {
    /// Native creates no Param object. This parent/attribute copy is diagnostic.
    pub parent: Option<BaseId>,
    pub attributes: Vec<StateGraphBinaryAttribute>,
    pub handled_by_override: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConstructionEvent {
    Constructed(BaseId),
    Attached {
        parent: BaseId,
        child: BaseId,
    },
    ReplacedExpression {
        parent: BaseId,
        previous: ExpressionId,
        replacement: ExpressionId,
    },
    FactoryCreated {
        kind: LeafKind,
        leaf: LeafId,
    },
    FactoryMiss {
        kind: LeafKind,
        name: Vec<u8>,
    },
    Param {
        parent: Option<BaseId>,
        handled: bool,
    },
    Unknown(Vec<u8>),
    Push {
        value: Option<BaseId>,
        depth: usize,
    },
    Pop {
        value: Option<BaseId>,
        depth: usize,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessEvent {
    State(StateId),
    Transition(TransitionId),
    Expression(ExpressionId),
    Leaf(LeafId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StateGraphFindError {
    InvalidStart(StateId),
    PrefixTooLong { length: usize },
}

impl fmt::Display for StateGraphFindError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStart(start) => {
                write!(
                    formatter,
                    "StateGraph Find start {start:?} is not in this arena"
                )
            }
            Self::PrefixTooLong { length } => write!(
                formatter,
                "StateGraph dotted Find prefix length {length} is not below 255"
            ),
        }
    }
}

impl Error for StateGraphFindError {}

pub struct LeafCreateRequest<'a> {
    pub leaf: LeafId,
    pub kind: LeafKind,
    pub parent: BaseId,
    pub requested_name: &'a [u8],
    pub registered_name: &'a [u8],
    pub attributes: &'a [StateGraphBinaryAttribute],
}

pub struct ParamRequest<'a, P> {
    pub leaf: LeafId,
    pub kind: LeafKind,
    pub registered_name: &'a [u8],
    pub payload: &'a mut P,
    pub attributes: &'a [StateGraphBinaryAttribute],
}

pub struct LeafProcessRequest<'a, P> {
    pub leaf: LeafId,
    pub kind: LeafKind,
    pub registered_name: &'a [u8],
    pub payload: &'a mut P,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaskedActivation {
    pub value: bool,
    pub was_masked: bool,
}

pub struct ConditionActivationRequest<'a, P, C> {
    pub leaf: LeafId,
    pub registered_name: &'a [u8],
    pub payload: &'a mut P,
    pub context: &'a C,
}

/// Runtime boundary for concrete `Condition::ActivationFunc` implementations.
///
/// The generic `StateGraph` layer owns enable/mask checks and expression
/// combination. This callback is reached only for an enabled Condition whose
/// stored mask intersects the requested mask.
pub trait StateGraphConditionActivationBackend<P, C> {
    type Error;

    /// Evaluates one concrete Condition's `ActivationFunc(Context const&)`.
    ///
    /// # Errors
    ///
    /// Returns the concrete backend's activation failure.
    fn activation_func(
        &mut self,
        request: ConditionActivationRequest<'_, P, C>,
    ) -> Result<bool, Self::Error>;
}

#[derive(Debug)]
pub enum StateGraphActivationError<E> {
    InvalidExpression(ExpressionId),
    InvalidState(StateId),
    InvalidTransition(TransitionId),
    InvalidLeaf(LeafId),
    NonConditionLeaf {
        leaf: LeafId,
        actual: LeafKind,
    },
    MissingConditionMask(LeafId),
    UninitializedCondition(LeafId),
    InvalidNotArity {
        expression: ExpressionId,
        children: usize,
    },
    Backend(E),
}

impl<E: fmt::Display> fmt::Display for StateGraphActivationError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidExpression(expression) => {
                write!(formatter, "invalid StateGraph Expression {expression:?}")
            }
            Self::InvalidState(state) => write!(formatter, "invalid StateGraph State {state:?}"),
            Self::InvalidTransition(transition) => {
                write!(formatter, "invalid StateGraph Transition {transition:?}")
            }
            Self::InvalidLeaf(leaf) => write!(formatter, "invalid StateGraph leaf {leaf:?}"),
            Self::NonConditionLeaf { leaf, actual } => write!(
                formatter,
                "StateGraph expression child {leaf:?} is {actual:?}, not a Condition"
            ),
            Self::MissingConditionMask(leaf) => {
                write!(formatter, "StateGraph Condition {leaf:?} has no mask")
            }
            Self::UninitializedCondition(leaf) => {
                write!(
                    formatter,
                    "StateGraph Condition {leaf:?} has no callback payload"
                )
            }
            Self::InvalidNotArity {
                expression,
                children,
            } => write!(
                formatter,
                "StateGraph NOT Expression {expression:?} has {children} children, expected one"
            ),
            Self::Backend(error) => {
                write!(formatter, "StateGraph activation callback failed: {error}")
            }
        }
    }
}

impl<E: Error + 'static> Error for StateGraphActivationError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Backend(error) => Some(error),
            Self::InvalidExpression(_)
            | Self::InvalidState(_)
            | Self::InvalidTransition(_)
            | Self::InvalidLeaf(_)
            | Self::NonConditionLeaf { .. }
            | Self::MissingConditionMask(_)
            | Self::UninitializedCondition(_)
            | Self::InvalidNotArity { .. } => None,
        }
    }
}

/// Synchronous boundary for the unrecovered concrete leaf classes.
///
/// Registration lists are searched by the core with ASCII case-insensitive
/// comparison, matching all shipped factory keys. Implementations must provide
/// a prevalidated C-string-domain list with no embedded NUL or ASCII-case-
/// equivalent duplicate; native registration and duplicate handling are not
/// implemented by this trait. `create` runs after the core has installed the
/// leaf's base and parent owning edge, but before descendants are visited. This
/// splits the native concrete constructor at its recovered base/derived
/// boundary while replacing intrusive ownership with arena IDs.
pub trait StateGraphLeafFactory {
    type Payload;
    type Error;

    fn registered_names(&self, kind: LeafKind) -> &[Vec<u8>];

    /// Constructs the concrete payload for an already attached leaf base.
    ///
    /// # Errors
    ///
    /// Returns the backend's concrete-constructor failure.
    fn create(&mut self, request: LeafCreateRequest<'_>) -> Result<Self::Payload, Self::Error>;

    /// Routes a `param` record to a concrete leaf override.
    ///
    /// # Errors
    ///
    /// Returns the backend's parameter handling failure.
    fn add_param(
        &mut self,
        _request: ParamRequest<'_, Self::Payload>,
    ) -> Result<bool, Self::Error> {
        Ok(false)
    }

    /// Invokes the opaque concrete leaf `Process` implementation.
    ///
    /// # Errors
    ///
    /// Returns the backend's leaf processing failure.
    fn process_leaf(
        &mut self,
        _request: LeafProcessRequest<'_, Self::Payload>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Debug)]
pub enum StateGraphRuntimeError<E> {
    CompiledStateGraphWrapper,
    UnsupportedInclude(Vec<u8>),
    MissingAttribute {
        element: Vec<u8>,
        key: &'static [u8],
    },
    TooManyAttributes {
        count: usize,
        maximum: usize,
    },
    InvalidAttributeHash {
        key: Vec<u8>,
        stored: u32,
        expected: u32,
    },
    EmbeddedNul {
        field: &'static str,
    },
    ParentAssertion {
        element: Vec<u8>,
        expected: &'static str,
    },
    StackCapacity {
        maximum: usize,
    },
    StackUnderflow,
    InvalidParserParent(StateId),
    InvalidOperator(Vec<u8>),
    InvalidConditionMask(Vec<u8>),
    FactoryMiss {
        kind: LeafKind,
        name: Vec<u8>,
    },
    Factory(E),
    BaseAddParam(BaseId),
    UninitializedLeaf(LeafId),
    MissingRoot,
    AlreadyProcessed,
    FindPrefixTooLong {
        length: usize,
    },
    InvalidInterruptTarget {
        state: StateId,
        name: Vec<u8>,
    },
    MissingTransitionTarget {
        transition: TransitionId,
        name: Vec<u8>,
    },
}

impl<E: fmt::Display> fmt::Display for StateGraphRuntimeError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CompiledStateGraphWrapper => formatter
                .write_str("compiled StateGraph payload used the XML-only stategraph wrapper"),
            Self::UnsupportedInclude(filename) => write!(
                formatter,
                "StateGraph include is unsupported at this binary-runtime boundary: {}",
                String::from_utf8_lossy(filename)
            ),
            Self::MissingAttribute { element, key } => write!(
                formatter,
                "StateGraph element {} is missing required attribute {}",
                String::from_utf8_lossy(element),
                String::from_utf8_lossy(key)
            ),
            Self::TooManyAttributes { count, maximum } => write!(
                formatter,
                "StateGraph runtime element has {count} attributes; maximum is {maximum}"
            ),
            Self::InvalidAttributeHash {
                key,
                stored,
                expected,
            } => write!(
                formatter,
                "StateGraph attribute {} stores hash {stored:08x}, expected {expected:08x}",
                String::from_utf8_lossy(key)
            ),
            Self::EmbeddedNul { field } => {
                write!(
                    formatter,
                    "StateGraph runtime {field} contains an embedded NUL"
                )
            }
            Self::ParentAssertion { element, expected } => write!(
                formatter,
                "StateGraph element {} requires {expected}",
                String::from_utf8_lossy(element)
            ),
            Self::StackCapacity { maximum } => {
                write!(
                    formatter,
                    "StateGraph runtime stack exceeds {maximum} entries"
                )
            }
            Self::StackUnderflow => formatter.write_str("StateGraph runtime stack underflow"),
            Self::InvalidParserParent(parent) => {
                write!(
                    formatter,
                    "StateGraph parser parent {parent:?} is not in this arena"
                )
            }
            Self::InvalidOperator(value) => write!(
                formatter,
                "invalid StateGraph expression operator {}",
                String::from_utf8_lossy(value)
            ),
            Self::InvalidConditionMask(value) => write!(
                formatter,
                "invalid StateGraph condition mask {}",
                String::from_utf8_lossy(value)
            ),
            Self::FactoryMiss { kind, name } => write!(
                formatter,
                "unregistered {kind:?} factory name {}",
                String::from_utf8_lossy(name)
            ),
            Self::Factory(error) => write!(formatter, "StateGraph leaf callback failed: {error}"),
            Self::BaseAddParam(parent) => {
                write!(
                    formatter,
                    "StateGraph Base::AddParam assertion for {parent:?}"
                )
            }
            Self::UninitializedLeaf(leaf) => {
                write!(
                    formatter,
                    "StateGraph leaf {leaf:?} has no callback payload"
                )
            }
            Self::MissingRoot => formatter.write_str("StateGraph has no root State"),
            Self::AlreadyProcessed => {
                formatter.write_str("StateGraph structural Process was already claimed")
            }
            Self::FindPrefixTooLong { length } => write!(
                formatter,
                "StateGraph dotted Find prefix length {length} is not below 255"
            ),
            Self::InvalidInterruptTarget { state, name } => write!(
                formatter,
                "StateGraph state {state:?} has invalid interrupt target {}",
                String::from_utf8_lossy(name)
            ),
            Self::MissingTransitionTarget { transition, name } => write!(
                formatter,
                "StateGraph transition {transition:?} cannot find target {}",
                String::from_utf8_lossy(name)
            ),
        }
    }
}

impl<E: Error + 'static> Error for StateGraphRuntimeError<E> {}

#[derive(Debug)]
pub struct StateGraphRuntime<P> {
    states: Vec<StateNode>,
    transitions: Vec<TransitionNode>,
    expressions: Vec<ExpressionNode>,
    leaves: Vec<LeafNode<P>>,
    params: Vec<ParamRecord>,
    root: Option<StateId>,
    construction_events: Vec<ConstructionEvent>,
    process_claimed: bool,
    process_succeeded: bool,
}

impl<P> Default for StateGraphRuntime<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P> StateGraphRuntime<P> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            states: Vec::new(),
            transitions: Vec::new(),
            expressions: Vec::new(),
            leaves: Vec::new(),
            params: Vec::new(),
            root: None,
            construction_events: Vec::new(),
            process_claimed: false,
            process_succeeded: false,
        }
    }

    pub(crate) const fn process_succeeded(&self) -> bool {
        self.process_succeeded
    }

    /// Constructs one compiled root without running structural `Process`.
    ///
    /// # Errors
    ///
    /// Returns a checked input, stack, structural, factory, or direct-field
    /// assertion equivalent.
    pub fn build<F>(
        element: &StateGraphBinaryElement,
        factory: &mut F,
        miss_policy: FactoryMissPolicy,
    ) -> Result<Self, StateGraphRuntimeError<F::Error>>
    where
        F: StateGraphLeafFactory<Payload = P>,
    {
        let mut graph = Self::new();
        graph.append(element, None, factory, miss_policy)?;
        Ok(graph)
    }

    /// Appends one decoded parser subtree with an optional State parent context.
    /// This does not model XML document/wrapper/include lifecycle.
    ///
    /// # Errors
    ///
    /// Returns a checked input, stack, structural, factory, or direct-field
    /// assertion equivalent. Constructor-time attachments may remain when an
    /// append into an existing graph fails, matching the native call order.
    pub fn append<F>(
        &mut self,
        element: &StateGraphBinaryElement,
        parser_parent: Option<StateId>,
        factory: &mut F,
        miss_policy: FactoryMissPolicy,
    ) -> Result<(), StateGraphRuntimeError<F::Error>>
    where
        F: StateGraphLeafFactory<Payload = P>,
    {
        if self.process_claimed {
            return Err(StateGraphRuntimeError::AlreadyProcessed);
        }
        if let Some(parent) = parser_parent
            && parent.0 >= self.states.len()
        {
            return Err(StateGraphRuntimeError::InvalidParserParent(parent));
        }
        let mut stack = Vec::with_capacity(STATE_GRAPH_PARSER_STACK_CAPACITY);
        self.visit(element, parser_parent, &mut stack, factory, miss_policy)
    }

    #[must_use]
    pub const fn root(&self) -> Option<StateId> {
        self.root
    }

    #[must_use]
    pub fn states(&self) -> &[StateNode] {
        &self.states
    }

    #[must_use]
    pub fn transitions(&self) -> &[TransitionNode] {
        &self.transitions
    }

    #[must_use]
    pub fn expressions(&self) -> &[ExpressionNode] {
        &self.expressions
    }

    #[must_use]
    pub fn leaves(&self) -> &[LeafNode<P>] {
        &self.leaves
    }

    pub(crate) fn leaf_callback_parts_mut(&mut self, leaf: LeafId) -> Option<(&[u8], &mut P)> {
        let node = self.leaves.get_mut(leaf.0)?;
        Some((&node.registered_name, node.payload.as_mut()?))
    }

    /// Evaluates one `ExpressionOperator` using the matched build's masked
    /// identities and short-circuit order.
    ///
    /// # Errors
    ///
    /// Returns a checked arena/leaf/NOT-arity error or a concrete Condition
    /// backend error.
    pub fn evaluate_expression_masked<B, C>(
        &mut self,
        expression: ExpressionId,
        mask: ConditionMask,
        context: &C,
        backend: &mut B,
    ) -> Result<MaskedActivation, StateGraphActivationError<B::Error>>
    where
        B: StateGraphConditionActivationBackend<P, C>,
    {
        self.evaluate_expression_masked_inner(expression, mask, context, backend)
    }

    /// Evaluates a State's directly attached expression. A missing or wholly
    /// masked expression activates, matching `Node::GetActivation`.
    ///
    /// # Errors
    ///
    /// Returns a checked arena/leaf/NOT-arity error or a concrete Condition
    /// backend error.
    pub fn state_activation<B, C>(
        &mut self,
        state: StateId,
        mask: ConditionMask,
        context: &C,
        backend: &mut B,
    ) -> Result<bool, StateGraphActivationError<B::Error>>
    where
        B: StateGraphConditionActivationBackend<P, C>,
    {
        let expression = self
            .states
            .get(state.0)
            .ok_or(StateGraphActivationError::InvalidState(state))?
            .expression;
        self.node_activation_inner(expression, mask, context, backend)
    }

    /// Evaluates a Transition's directly attached expression. A missing or
    /// wholly masked expression activates, matching `Node::GetActivation`.
    ///
    /// # Errors
    ///
    /// Returns a checked arena/leaf/NOT-arity error or a concrete Condition
    /// backend error.
    pub fn transition_activation<B, C>(
        &mut self,
        transition: TransitionId,
        mask: ConditionMask,
        context: &C,
        backend: &mut B,
    ) -> Result<bool, StateGraphActivationError<B::Error>>
    where
        B: StateGraphConditionActivationBackend<P, C>,
    {
        let expression = self
            .transitions
            .get(transition.0)
            .ok_or(StateGraphActivationError::InvalidTransition(transition))?
            .expression;
        self.node_activation_inner(expression, mask, context, backend)
    }

    /// Evaluates a State first and then each State ancestor, stopping at the
    /// first false activation.
    ///
    /// # Errors
    ///
    /// Returns a checked arena/leaf/NOT-arity error or a concrete Condition
    /// backend error.
    pub fn state_activation_parent<B, C>(
        &mut self,
        state: StateId,
        mask: ConditionMask,
        context: &C,
        backend: &mut B,
    ) -> Result<bool, StateGraphActivationError<B::Error>>
    where
        B: StateGraphConditionActivationBackend<P, C>,
    {
        let mut current = Some(state);
        while let Some(candidate) = current {
            let node = self
                .states
                .get(candidate.0)
                .ok_or(StateGraphActivationError::InvalidState(candidate))?;
            let expression = node.expression;
            let parent = node.parent_state;
            if !self.node_activation_inner(expression, mask, context, backend)? {
                return Ok(false);
            }
            current = parent;
        }
        Ok(true)
    }

    #[must_use]
    pub fn params(&self) -> &[ParamRecord] {
        &self.params
    }

    #[must_use]
    pub fn construction_events(&self) -> &[ConstructionEvent] {
        &self.construction_events
    }

    fn node_activation_inner<B, C>(
        &mut self,
        expression: Option<ExpressionId>,
        mask: ConditionMask,
        context: &C,
        backend: &mut B,
    ) -> Result<bool, StateGraphActivationError<B::Error>>
    where
        B: StateGraphConditionActivationBackend<P, C>,
    {
        let Some(expression) = expression else {
            return Ok(true);
        };
        let activation =
            self.evaluate_expression_masked_inner(expression, mask, context, backend)?;
        Ok(activation.was_masked || activation.value)
    }

    fn evaluate_expression_masked_inner<B, C>(
        &mut self,
        expression: ExpressionId,
        mask: ConditionMask,
        context: &C,
        backend: &mut B,
    ) -> Result<MaskedActivation, StateGraphActivationError<B::Error>>
    where
        B: StateGraphConditionActivationBackend<P, C>,
    {
        let node = self
            .expressions
            .get(expression.0)
            .ok_or(StateGraphActivationError::InvalidExpression(expression))?;
        let operator = node.operator;
        let children = node.children.clone();

        match operator {
            ExpressionOperator::None | ExpressionOperator::And => {
                let mut value = true;
                let mut all_masked = true;
                for child in children {
                    if !value {
                        break;
                    }
                    let child = self.evaluate_expression_child(child, mask, context, backend)?;
                    if !child.was_masked {
                        all_masked = false;
                        value &= child.value;
                    }
                }
                if all_masked {
                    value = true;
                }
                Ok(MaskedActivation {
                    value,
                    was_masked: all_masked,
                })
            }
            ExpressionOperator::Or => {
                let mut value = false;
                let mut all_masked = true;
                for child in children {
                    if value {
                        break;
                    }
                    let child = self.evaluate_expression_child(child, mask, context, backend)?;
                    if !child.was_masked {
                        all_masked = false;
                        value = child.value;
                    }
                }
                if all_masked {
                    value = true;
                }
                Ok(MaskedActivation {
                    value,
                    was_masked: all_masked,
                })
            }
            ExpressionOperator::Not => {
                if children.len() != 1 {
                    return Err(StateGraphActivationError::InvalidNotArity {
                        expression,
                        children: children.len(),
                    });
                }
                let child = self.evaluate_expression_child(children[0], mask, context, backend)?;
                if child.was_masked {
                    Ok(MaskedActivation {
                        value: true,
                        was_masked: true,
                    })
                } else {
                    Ok(MaskedActivation {
                        value: !child.value,
                        was_masked: false,
                    })
                }
            }
        }
    }

    fn evaluate_expression_child<B, C>(
        &mut self,
        child: ExpressionChildId,
        mask: ConditionMask,
        context: &C,
        backend: &mut B,
    ) -> Result<MaskedActivation, StateGraphActivationError<B::Error>>
    where
        B: StateGraphConditionActivationBackend<P, C>,
    {
        match child {
            ExpressionChildId::Expression(child) => {
                self.evaluate_expression_masked_inner(child, mask, context, backend)
            }
            ExpressionChildId::Condition(child) => {
                self.evaluate_condition_masked(child, mask, context, backend)
            }
        }
    }

    fn evaluate_condition_masked<B, C>(
        &mut self,
        leaf: LeafId,
        requested_mask: ConditionMask,
        context: &C,
        backend: &mut B,
    ) -> Result<MaskedActivation, StateGraphActivationError<B::Error>>
    where
        B: StateGraphConditionActivationBackend<P, C>,
    {
        let condition = self
            .leaves
            .get_mut(leaf.0)
            .ok_or(StateGraphActivationError::InvalidLeaf(leaf))?;
        if condition.kind != LeafKind::Condition {
            return Err(StateGraphActivationError::NonConditionLeaf {
                leaf,
                actual: condition.kind,
            });
        }
        let stored_mask = condition
            .mask
            .ok_or(StateGraphActivationError::MissingConditionMask(leaf))?;
        if !condition.base.enabled || (stored_mask as i32 & requested_mask as i32) == 0 {
            return Ok(MaskedActivation {
                value: true,
                was_masked: true,
            });
        }
        let payload = condition
            .payload
            .as_mut()
            .ok_or(StateGraphActivationError::UninitializedCondition(leaf))?;
        let value = backend
            .activation_func(ConditionActivationRequest {
                leaf,
                registered_name: &condition.registered_name,
                payload,
                context,
            })
            .map_err(StateGraphActivationError::Backend)?;
        Ok(MaskedActivation {
            value,
            was_masked: false,
        })
    }

    fn visit<F>(
        &mut self,
        element: &StateGraphBinaryElement,
        parser_parent: Option<StateId>,
        stack: &mut Vec<Option<BaseId>>,
        factory: &mut F,
        miss_policy: FactoryMissPolicy,
    ) -> Result<(), StateGraphRuntimeError<F::Error>>
    where
        F: StateGraphLeafFactory<Payload = P>,
    {
        validate_element(element)?;
        let value = self.process_element(element, parser_parent, stack, factory, miss_policy)?;
        if stack.len() == STATE_GRAPH_PARSER_STACK_CAPACITY {
            return Err(StateGraphRuntimeError::StackCapacity {
                maximum: STATE_GRAPH_PARSER_STACK_CAPACITY,
            });
        }
        stack.push(value);
        self.construction_events.push(ConstructionEvent::Push {
            value,
            depth: stack.len(),
        });
        for child in &element.children {
            self.visit(child, parser_parent, stack, factory, miss_policy)?;
        }
        let popped = stack.pop().ok_or(StateGraphRuntimeError::StackUnderflow)?;
        self.construction_events.push(ConstructionEvent::Pop {
            value: popped,
            depth: stack.len(),
        });
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn process_element<F>(
        &mut self,
        element: &StateGraphBinaryElement,
        parser_parent: Option<StateId>,
        stack: &[Option<BaseId>],
        factory: &mut F,
        miss_policy: FactoryMissPolicy,
    ) -> Result<Option<BaseId>, StateGraphRuntimeError<F::Error>>
    where
        F: StateGraphLeafFactory<Payload = P>,
    {
        let kind = tag_kind(&element.name);
        if kind == Some(TagKind::StateGraph) {
            return Err(StateGraphRuntimeError::CompiledStateGraphWrapper);
        }
        let contexts = Self::contexts(stack, parser_parent);
        let value = match kind {
            Some(TagKind::State) => {
                Some(BaseId::State(self.construct_state(element, contexts.state)))
            }
            Some(TagKind::Transition) => {
                let parent = contexts
                    .state
                    .ok_or_else(|| parent_error(element, "State parent"))?;
                Some(BaseId::Transition(
                    self.construct_transition(element, parent)?,
                ))
            }
            Some(TagKind::Expression) => {
                let top = contexts
                    .top
                    .ok_or_else(|| parent_error(element, "non-null stack top"))?;
                let parent = if let Some(node) = contexts.node {
                    ExpressionParent::Node(node)
                } else if let Some(expression) = contexts.expression {
                    ExpressionParent::Expression(expression)
                } else {
                    return Err(parent_error(element, "Node or ExpressionOperator parent"));
                };
                debug_assert_eq!(Some(top), Some(parent.base_id()));
                Some(BaseId::Expression(
                    self.construct_expression(element, parent)?,
                ))
            }
            Some(TagKind::Condition) => {
                let parent = contexts
                    .expression
                    .ok_or_else(|| parent_error(element, "ExpressionOperator parent"))?;
                self.construct_leaf(
                    element,
                    BaseId::Expression(parent),
                    LeafKind::Condition,
                    factory,
                    miss_policy,
                )?
                .map(BaseId::Leaf)
            }
            Some(TagKind::Behaviour) => {
                let parent = contexts
                    .state
                    .ok_or_else(|| parent_error(element, "State parent"))?;
                self.construct_leaf(
                    element,
                    BaseId::State(parent),
                    LeafKind::Behaviour,
                    factory,
                    miss_policy,
                )?
                .map(BaseId::Leaf)
            }
            Some(TagKind::Param) => {
                self.add_param(element, contexts.top, factory)?;
                None
            }
            Some(TagKind::Hook) => {
                let parent = contexts
                    .transition
                    .ok_or_else(|| parent_error(element, "Transition parent"))?;
                self.construct_leaf(
                    element,
                    BaseId::Transition(parent),
                    LeafKind::Hook,
                    factory,
                    miss_policy,
                )?
                .map(BaseId::Leaf)
            }
            Some(TagKind::Include) => {
                let filename = required_text(element, b"filename")?.to_vec();
                contexts
                    .state
                    .ok_or_else(|| parent_error(element, "State parent"))?;
                return Err(StateGraphRuntimeError::UnsupportedInclude(filename));
            }
            Some(TagKind::StateGraph) => unreachable!("handled before context construction"),
            None => {
                self.construction_events
                    .push(ConstructionEvent::Unknown(element.name.clone()));
                None
            }
        };
        Ok(value)
    }

    fn contexts(stack: &[Option<BaseId>], parser_parent: Option<StateId>) -> Contexts {
        let Some(top) = stack.last() else {
            return Contexts {
                state: parser_parent,
                ..Contexts::default()
            };
        };
        let Some(top) = *top else {
            return Contexts::default();
        };
        match top {
            BaseId::State(state) => Contexts {
                top: Some(top),
                state: Some(state),
                node: Some(NodeId::State(state)),
                ..Contexts::default()
            },
            BaseId::Transition(transition) => Contexts {
                top: Some(top),
                transition: Some(transition),
                node: Some(NodeId::Transition(transition)),
                ..Contexts::default()
            },
            BaseId::Expression(expression) => Contexts {
                top: Some(top),
                expression: Some(expression),
                ..Contexts::default()
            },
            BaseId::Leaf(_) => Contexts {
                top: Some(top),
                ..Contexts::default()
            },
        }
    }

    fn construct_state(
        &mut self,
        element: &StateGraphBinaryElement,
        parent: Option<StateId>,
    ) -> StateId {
        let parent_base = parent.map(BaseId::State);
        let base = self.make_base(element, parent_base);
        let depth = parent.map_or(0, |parent| self.states[parent.0].depth.wrapping_add(1));
        let id = StateId(self.states.len());
        self.states.push(StateNode {
            base,
            parent_state: parent,
            expression: None,
            active: true,
            interruptable: Interruptable::RootInterruptable,
            children: Vec::new(),
            behaviours: Vec::new(),
            transitions: Vec::new(),
            depth,
        });
        self.construction_events
            .push(ConstructionEvent::Constructed(BaseId::State(id)));
        if let Some(parent) = parent {
            self.states[parent.0].children.push(id);
            self.record_attachment(BaseId::State(parent), BaseId::State(id));
        }
        self.states[id.0].active = optional_bool(element, b"active", true);
        self.states[id.0].interruptable = parse_interruptable(element);
        if self.root.is_none() {
            self.root = Some(id);
        }
        id
    }

    fn construct_transition<E>(
        &mut self,
        element: &StateGraphBinaryElement,
        parent: StateId,
    ) -> Result<TransitionId, StateGraphRuntimeError<E>> {
        let base = self.make_base(element, Some(BaseId::State(parent)));
        let id = TransitionId(self.transitions.len());
        self.transitions.push(TransitionNode {
            base,
            parent_state: parent,
            expression: None,
            target_name: Vec::new(),
            target: None,
            priority: TransitionPriority::Low,
            hooks: Vec::new(),
        });
        self.construction_events
            .push(ConstructionEvent::Constructed(BaseId::Transition(id)));
        self.states[parent.0].transitions.push(id);
        self.record_attachment(BaseId::State(parent), BaseId::Transition(id));
        self.transitions[id.0].target_name = required_text(element, b"target")?.to_vec();
        self.transitions[id.0].priority = parse_priority(element);
        Ok(id)
    }

    fn construct_expression<E>(
        &mut self,
        element: &StateGraphBinaryElement,
        parent: ExpressionParent,
    ) -> Result<ExpressionId, StateGraphRuntimeError<E>> {
        let parent_base = parent.base_id();
        let base = self.make_base(element, Some(parent_base));
        let id = ExpressionId(self.expressions.len());
        self.expressions.push(ExpressionNode {
            base,
            operator: ExpressionOperator::None,
            children: Vec::new(),
        });
        self.construction_events
            .push(ConstructionEvent::Constructed(BaseId::Expression(id)));
        match parent {
            ExpressionParent::Node(NodeId::State(state)) => {
                if let Some(previous) = self.states[state.0].expression.replace(id) {
                    self.record_replacement(BaseId::State(state), previous, id);
                }
            }
            ExpressionParent::Node(NodeId::Transition(transition)) => {
                if let Some(previous) = self.transitions[transition.0].expression.replace(id) {
                    self.record_replacement(BaseId::Transition(transition), previous, id);
                }
            }
            ExpressionParent::Expression(expression) => {
                self.expressions[expression.0]
                    .children
                    .push(ExpressionChildId::Expression(id));
            }
        }
        self.record_attachment(parent_base, BaseId::Expression(id));
        self.expressions[id.0].operator = parse_operator(element)?;
        Ok(id)
    }

    fn construct_leaf<F>(
        &mut self,
        element: &StateGraphBinaryElement,
        parent: BaseId,
        kind: LeafKind,
        factory: &mut F,
        miss_policy: FactoryMissPolicy,
    ) -> Result<Option<LeafId>, StateGraphRuntimeError<F::Error>>
    where
        F: StateGraphLeafFactory<Payload = P>,
    {
        let requested_name = required_text(element, b"name")?;
        let registered_name = factory
            .registered_names(kind)
            .iter()
            .find(|name| name.as_slice().eq_ignore_ascii_case(requested_name))
            .cloned();
        let Some(registered_name) = registered_name else {
            self.construction_events
                .push(ConstructionEvent::FactoryMiss {
                    kind,
                    name: requested_name.to_vec(),
                });
            return match miss_policy {
                FactoryMissPolicy::Strict => Err(StateGraphRuntimeError::FactoryMiss {
                    kind,
                    name: requested_name.to_vec(),
                }),
                FactoryMissPolicy::NativeSafeNull => Ok(None),
            };
        };
        let base = self.make_base(element, Some(parent));
        let id = LeafId(self.leaves.len());
        self.leaves.push(LeafNode {
            base,
            kind,
            registered_name,
            mask: None,
            payload: None,
        });
        self.construction_events
            .push(ConstructionEvent::Constructed(BaseId::Leaf(id)));
        self.attach_leaf(parent, id, kind);
        self.record_attachment(parent, BaseId::Leaf(id));
        if kind == LeafKind::Condition {
            self.leaves[id.0].mask = Some(parse_condition_mask(element)?);
        }
        let registered_name = self.leaves[id.0].registered_name.clone();
        let payload = factory
            .create(LeafCreateRequest {
                leaf: id,
                kind,
                parent,
                requested_name,
                registered_name: &registered_name,
                attributes: &element.attributes,
            })
            .map_err(StateGraphRuntimeError::Factory)?;
        self.leaves[id.0].payload = Some(payload);
        self.construction_events
            .push(ConstructionEvent::FactoryCreated { kind, leaf: id });
        Ok(Some(id))
    }

    fn add_param<F>(
        &mut self,
        element: &StateGraphBinaryElement,
        parent: Option<BaseId>,
        factory: &mut F,
    ) -> Result<(), StateGraphRuntimeError<F::Error>>
    where
        F: StateGraphLeafFactory<Payload = P>,
    {
        let handled = match parent {
            None => false,
            Some(BaseId::Leaf(leaf)) => {
                let node = &mut self.leaves[leaf.0];
                let payload = node
                    .payload
                    .as_mut()
                    .ok_or(StateGraphRuntimeError::UninitializedLeaf(leaf))?;
                factory
                    .add_param(ParamRequest {
                        leaf,
                        kind: node.kind,
                        registered_name: &node.registered_name,
                        payload,
                        attributes: &element.attributes,
                    })
                    .map_err(StateGraphRuntimeError::Factory)?
            }
            Some(parent) => return Err(StateGraphRuntimeError::BaseAddParam(parent)),
        };
        if let Some(parent) = parent
            && !handled
        {
            return Err(StateGraphRuntimeError::BaseAddParam(parent));
        }
        self.params.push(ParamRecord {
            parent,
            attributes: element.attributes.clone(),
            handled_by_override: handled,
        });
        self.construction_events
            .push(ConstructionEvent::Param { parent, handled });
        Ok(())
    }

    fn make_base(&self, element: &StateGraphBinaryElement, parent: Option<BaseId>) -> BaseFields {
        let name = optional_text(element, b"name", b"__unknown__").to_vec();
        let parent_enabled = parent.is_none_or(|parent| self.base(parent).enabled);
        let enabled = parent_enabled && optional_bool(element, b"enabled", true);
        let path = parent.map_or_else(
            || name.clone(),
            |parent| {
                let parent_path = &self.base(parent).path;
                let mut path = Vec::with_capacity(parent_path.len() + 1 + name.len());
                path.extend_from_slice(parent_path);
                path.push(b'.');
                path.extend_from_slice(&name);
                path
            },
        );
        BaseFields {
            name,
            path,
            parent,
            enabled,
            cast_parent: parent.is_none(),
        }
    }

    fn attach_leaf(&mut self, parent: BaseId, leaf: LeafId, kind: LeafKind) {
        match (parent, kind) {
            (BaseId::Expression(expression), LeafKind::Condition) => self.expressions[expression.0]
                .children
                .push(ExpressionChildId::Condition(leaf)),
            (BaseId::State(state), LeafKind::Behaviour) => {
                self.states[state.0].behaviours.push(leaf);
            }
            (BaseId::Transition(transition), LeafKind::Hook) => {
                self.transitions[transition.0].hooks.push(leaf);
            }
            _ => unreachable!("ProcessElement validated leaf parent kind"),
        }
    }

    fn record_attachment(&mut self, parent: BaseId, child: BaseId) {
        self.construction_events
            .push(ConstructionEvent::Attached { parent, child });
    }

    fn record_replacement(
        &mut self,
        parent: BaseId,
        previous: ExpressionId,
        replacement: ExpressionId,
    ) {
        self.construction_events
            .push(ConstructionEvent::ReplacedExpression {
                parent,
                previous,
                replacement,
            });
    }

    fn base(&self, id: BaseId) -> &BaseFields {
        match id {
            BaseId::State(id) => &self.states[id.0].base,
            BaseId::Transition(id) => &self.transitions[id.0].base,
            BaseId::Expression(id) => &self.expressions[id.0].base,
            BaseId::Leaf(id) => &self.leaves[id.0].base,
        }
    }
}

impl<P> StateGraphRuntime<P> {
    /// Reconstructs `State::Find(name, search_upwards)` over arena IDs.
    ///
    /// # Errors
    ///
    /// Returns a checked error for a foreign start ID or the native dotted-
    /// prefix assertion (`prefix length < 255`).
    pub fn find_state(
        &self,
        start: StateId,
        name: &[u8],
        search_upwards: bool,
    ) -> Result<Option<StateId>, StateGraphFindError> {
        if start.0 >= self.states.len() {
            return Err(StateGraphFindError::InvalidStart(start));
        }
        if let Some(dot) = name.iter().rposition(|byte| *byte == b'.') {
            if dot >= 255 {
                return Err(StateGraphFindError::PrefixTooLong { length: dot });
            }
            let Some(prefix) = self.find_state(start, &name[..dot], true)? else {
                return Ok(None);
            };
            return Ok(self.find_simple(prefix, &name[dot + 1..], false));
        }
        Ok(self.find_simple(start, name, search_upwards))
    }

    /// Runs the proven structural root `Process` traversal once for this loader.
    ///
    /// Leaf callbacks are opaque: success proves structural resolution and call
    /// order, not gameplay condition/behaviour/hook semantics. The one-shot
    /// claim is checked-host loader policy, not a recovered latch inside native
    /// `State::Process`.
    ///
    /// # Errors
    ///
    /// Returns a missing/invalid target, repeated Process claim, uninitialized
    /// leaf, dotted-Find assertion equivalent, or leaf callback failure.
    pub fn process_after_build<F>(
        &mut self,
        factory: &mut F,
    ) -> Result<Vec<ProcessEvent>, StateGraphRuntimeError<F::Error>>
    where
        F: StateGraphLeafFactory<Payload = P>,
    {
        let root = self.root.ok_or(StateGraphRuntimeError::MissingRoot)?;
        if self.process_claimed {
            return Err(StateGraphRuntimeError::AlreadyProcessed);
        }
        // A failed callback may already have produced concrete effects. Claim
        // before traversal so checked-host retry cannot duplicate them.
        self.process_claimed = true;
        let mut events = Vec::new();
        self.process_state(root, factory, &mut events)?;
        self.process_succeeded = true;
        Ok(events)
    }

    fn find_simple(&self, start: StateId, name: &[u8], search_upwards: bool) -> Option<StateId> {
        let state = &self.states[start.0];
        for child in &state.children {
            if self.states[child.0].base.name == name {
                return Some(*child);
            }
        }
        if !search_upwards {
            return None;
        }
        if state.base.name == name {
            return Some(start);
        }
        state
            .parent_state
            .and_then(|parent| self.find_simple(parent, name, true))
    }

    fn process_state<F>(
        &mut self,
        state: StateId,
        factory: &mut F,
        events: &mut Vec<ProcessEvent>,
    ) -> Result<(), StateGraphRuntimeError<F::Error>>
    where
        F: StateGraphLeafFactory<Payload = P>,
    {
        events.push(ProcessEvent::State(state));
        let named_target = match &self.states[state.0].interruptable {
            Interruptable::StateInterruptable { target_name, .. } => Some(target_name.clone()),
            Interruptable::NotInterruptable | Interruptable::RootInterruptable => None,
        };
        if let Some(name) = named_target {
            let target = self
                .find_state(state, &name, true)
                .map_err(|error| match error {
                    StateGraphFindError::PrefixTooLong { length } => {
                        StateGraphRuntimeError::FindPrefixTooLong { length }
                    }
                    StateGraphFindError::InvalidStart(_) => {
                        unreachable!("Process owns its State ID")
                    }
                })?
                .filter(|target| self.is_self_or_ancestor(state, *target))
                .ok_or_else(|| StateGraphRuntimeError::InvalidInterruptTarget {
                    state,
                    name: name.clone(),
                })?;
            if let Interruptable::StateInterruptable {
                target: stored_target,
                ..
            } = &mut self.states[state.0].interruptable
            {
                *stored_target = Some(target);
            }
        }
        if let Some(expression) = self.states[state.0].expression {
            self.process_expression(expression, factory, events)?;
        }
        let children = self.states[state.0].children.clone();
        for child in children {
            self.process_state(child, factory, events)?;
        }
        let behaviours = self.states[state.0].behaviours.clone();
        for behaviour in behaviours {
            self.process_leaf(behaviour, factory, events)?;
        }
        let transitions = self.states[state.0].transitions.clone();
        for transition in transitions {
            self.process_transition(transition, factory, events)?;
        }
        Ok(())
    }

    fn process_transition<F>(
        &mut self,
        transition: TransitionId,
        factory: &mut F,
        events: &mut Vec<ProcessEvent>,
    ) -> Result<(), StateGraphRuntimeError<F::Error>>
    where
        F: StateGraphLeafFactory<Payload = P>,
    {
        events.push(ProcessEvent::Transition(transition));
        if let Some(expression) = self.transitions[transition.0].expression {
            self.process_expression(expression, factory, events)?;
        }
        let parent = self.transitions[transition.0].parent_state;
        let name = self.transitions[transition.0].target_name.clone();
        let target = self
            .find_state(parent, &name, true)
            .map_err(|error| match error {
                StateGraphFindError::PrefixTooLong { length } => {
                    StateGraphRuntimeError::FindPrefixTooLong { length }
                }
                StateGraphFindError::InvalidStart(_) => {
                    unreachable!("Transition owns its parent State ID")
                }
            })?
            .ok_or_else(|| StateGraphRuntimeError::MissingTransitionTarget { transition, name })?;
        self.transitions[transition.0].target = Some(target);
        if !self.states[target.0].base.enabled {
            self.transitions[transition.0].base.enabled = false;
        }
        let hooks = self.transitions[transition.0].hooks.clone();
        for hook in hooks {
            self.process_leaf(hook, factory, events)?;
        }
        Ok(())
    }

    fn process_expression<F>(
        &mut self,
        expression: ExpressionId,
        factory: &mut F,
        events: &mut Vec<ProcessEvent>,
    ) -> Result<(), StateGraphRuntimeError<F::Error>>
    where
        F: StateGraphLeafFactory<Payload = P>,
    {
        events.push(ProcessEvent::Expression(expression));
        let children = self.expressions[expression.0].children.clone();
        for child in children {
            match child {
                ExpressionChildId::Expression(child) => {
                    self.process_expression(child, factory, events)?;
                }
                ExpressionChildId::Condition(child) => {
                    self.process_leaf(child, factory, events)?;
                }
            }
        }
        Ok(())
    }

    fn process_leaf<F>(
        &mut self,
        leaf: LeafId,
        factory: &mut F,
        events: &mut Vec<ProcessEvent>,
    ) -> Result<(), StateGraphRuntimeError<F::Error>>
    where
        F: StateGraphLeafFactory<Payload = P>,
    {
        events.push(ProcessEvent::Leaf(leaf));
        let node = &mut self.leaves[leaf.0];
        let payload = node
            .payload
            .as_mut()
            .ok_or(StateGraphRuntimeError::UninitializedLeaf(leaf))?;
        factory
            .process_leaf(LeafProcessRequest {
                leaf,
                kind: node.kind,
                registered_name: &node.registered_name,
                payload,
            })
            .map_err(StateGraphRuntimeError::Factory)
    }

    fn is_self_or_ancestor(&self, state: StateId, target: StateId) -> bool {
        let mut cursor = Some(state);
        while let Some(current) = cursor {
            if current == target {
                return true;
            }
            cursor = self.states[current.0].parent_state;
        }
        false
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NodeId {
    State(StateId),
    Transition(TransitionId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExpressionParent {
    Node(NodeId),
    Expression(ExpressionId),
}

impl ExpressionParent {
    const fn base_id(self) -> BaseId {
        match self {
            Self::Node(NodeId::State(id)) => BaseId::State(id),
            Self::Node(NodeId::Transition(id)) => BaseId::Transition(id),
            Self::Expression(id) => BaseId::Expression(id),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Contexts {
    top: Option<BaseId>,
    state: Option<StateId>,
    node: Option<NodeId>,
    transition: Option<TransitionId>,
    expression: Option<ExpressionId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TagKind {
    StateGraph,
    State,
    Transition,
    Expression,
    Condition,
    Behaviour,
    Param,
    Hook,
    Include,
}

fn tag_kind(name: &[u8]) -> Option<TagKind> {
    match state_graph_string_hash32(name) {
        0x7526_fd2f => Some(TagKind::StateGraph),
        0x5f91_f83d => Some(TagKind::State),
        0x8c0e_fa95 => Some(TagKind::Transition),
        0xf617_65b1 => Some(TagKind::Expression),
        0xf868_f111 => Some(TagKind::Condition),
        0xae36_6e1e => Some(TagKind::Behaviour),
        0xe54f_b484 => Some(TagKind::Param),
        0x18c2_92cc => Some(TagKind::Hook),
        0x902a_b1f7 => Some(TagKind::Include),
        _ => None,
    }
}

fn parent_error<E>(
    element: &StateGraphBinaryElement,
    expected: &'static str,
) -> StateGraphRuntimeError<E> {
    StateGraphRuntimeError::ParentAssertion {
        element: element.name.clone(),
        expected,
    }
}

fn validate_element<E>(element: &StateGraphBinaryElement) -> Result<(), StateGraphRuntimeError<E>> {
    if element.attributes.len() > STATE_GRAPH_BINARY_ATTRIBUTE_CAPACITY {
        return Err(StateGraphRuntimeError::TooManyAttributes {
            count: element.attributes.len(),
            maximum: STATE_GRAPH_BINARY_ATTRIBUTE_CAPACITY,
        });
    }
    if element.name.contains(&0) {
        return Err(StateGraphRuntimeError::EmbeddedNul {
            field: "element name",
        });
    }
    for attribute in &element.attributes {
        if attribute.key.contains(&0) {
            return Err(StateGraphRuntimeError::EmbeddedNul {
                field: "attribute key",
            });
        }
        if attribute.text.contains(&0) {
            return Err(StateGraphRuntimeError::EmbeddedNul {
                field: "attribute text",
            });
        }
        let expected = state_graph_string_hash32(&attribute.key);
        if attribute.key_hash != expected {
            return Err(StateGraphRuntimeError::InvalidAttributeHash {
                key: attribute.key.clone(),
                stored: attribute.key_hash,
                expected,
            });
        }
    }
    Ok(())
}

fn required_text<'a, E>(
    element: &'a StateGraphBinaryElement,
    key: &'static [u8],
) -> Result<&'a [u8], StateGraphRuntimeError<E>> {
    element
        .attribute(key)
        .map(|attribute| attribute.text.as_slice())
        .ok_or_else(|| StateGraphRuntimeError::MissingAttribute {
            element: element.name.clone(),
            key,
        })
}

fn optional_text<'a>(
    element: &'a StateGraphBinaryElement,
    key: &[u8],
    default: &'a [u8],
) -> &'a [u8] {
    element
        .attribute(key)
        .map_or(default, |attribute| attribute.text.as_slice())
}

fn optional_bool(element: &StateGraphBinaryElement, key: &[u8], default: bool) -> bool {
    element
        .attribute(key)
        .map_or(default, |attribute| attribute.boolean_raw != 0)
}

fn parse_interruptable(element: &StateGraphBinaryElement) -> Interruptable {
    match optional_text(element, b"interruptable", b"true") {
        b"true" => Interruptable::RootInterruptable,
        b"false" => Interruptable::NotInterruptable,
        target => Interruptable::StateInterruptable {
            target_name: target.to_vec(),
            target: None,
        },
    }
}

fn parse_priority(element: &StateGraphBinaryElement) -> TransitionPriority {
    match element
        .attribute(b"priority")
        .map(|attribute| attribute.text.as_slice())
    {
        Some(b"med") => TransitionPriority::Medium,
        Some(b"high") => TransitionPriority::High,
        Some(b"urgent") => TransitionPriority::Urgent,
        _ => TransitionPriority::Low,
    }
}

fn parse_operator<E>(
    element: &StateGraphBinaryElement,
) -> Result<ExpressionOperator, StateGraphRuntimeError<E>> {
    match element
        .attribute(b"op")
        .map(|attribute| attribute.text.as_slice())
    {
        None => Ok(ExpressionOperator::None),
        Some(b"and") => Ok(ExpressionOperator::And),
        Some(b"or") => Ok(ExpressionOperator::Or),
        Some(b"not") => Ok(ExpressionOperator::Not),
        Some(value) => Err(StateGraphRuntimeError::InvalidOperator(value.to_vec())),
    }
}

fn parse_condition_mask<E>(
    element: &StateGraphBinaryElement,
) -> Result<ConditionMask, StateGraphRuntimeError<E>> {
    match element
        .attribute(b"mask")
        .map(|attribute| attribute.text.as_slice())
    {
        None | Some(b"precond") => Ok(ConditionMask::PreCond),
        Some(b"sustain") => Ok(ConditionMask::Sustain),
        Some(b"always") => Ok(ConditionMask::Always),
        Some(b"postcond") => Ok(ConditionMask::PostCond),
        Some(value) => Err(StateGraphRuntimeError::InvalidConditionMask(value.to_vec())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::Infallible;

    #[derive(Default)]
    struct TestFactory {
        conditions: Vec<Vec<u8>>,
        behaviours: Vec<Vec<u8>>,
        hooks: Vec<Vec<u8>>,
        created: Vec<(LeafKind, LeafId)>,
        params: Vec<LeafId>,
        processed: Vec<LeafId>,
    }

    impl TestFactory {
        fn shipped_slice() -> Self {
            Self {
                conditions: vec![b"Cond".to_vec()],
                behaviours: vec![b"Beh".to_vec()],
                hooks: vec![b"Hook".to_vec()],
                ..Self::default()
            }
        }
    }

    impl StateGraphLeafFactory for TestFactory {
        type Payload = Vec<u8>;
        type Error = Infallible;

        fn registered_names(&self, kind: LeafKind) -> &[Vec<u8>] {
            match kind {
                LeafKind::Condition => &self.conditions,
                LeafKind::Behaviour => &self.behaviours,
                LeafKind::Hook => &self.hooks,
            }
        }

        fn create(&mut self, request: LeafCreateRequest<'_>) -> Result<Self::Payload, Self::Error> {
            self.created.push((request.kind, request.leaf));
            Ok(request.registered_name.to_vec())
        }

        fn add_param(
            &mut self,
            request: ParamRequest<'_, Self::Payload>,
        ) -> Result<bool, Self::Error> {
            if request.kind == LeafKind::Behaviour {
                self.params.push(request.leaf);
                request.payload.extend_from_slice(b"+param");
                Ok(true)
            } else {
                Ok(false)
            }
        }

        fn process_leaf(
            &mut self,
            request: LeafProcessRequest<'_, Self::Payload>,
        ) -> Result<(), Self::Error> {
            self.processed.push(request.leaf);
            Ok(())
        }
    }

    #[derive(Default)]
    struct TestActivationBackend {
        calls: Vec<LeafId>,
    }

    impl StateGraphConditionActivationBackend<Vec<u8>, ()> for TestActivationBackend {
        type Error = Infallible;

        fn activation_func(
            &mut self,
            request: ConditionActivationRequest<'_, Vec<u8>, ()>,
        ) -> Result<bool, Self::Error> {
            self.calls.push(request.leaf);
            Ok(match request.registered_name {
                b"True" | b"Later" | b"RootTrue" | b"ChildTrue" => true,
                b"False" | b"RootFalse" | b"ChildFalse" => false,
                unexpected => panic!(
                    "unexpected activation payload {}",
                    String::from_utf8_lossy(unexpected)
                ),
            })
        }
    }

    struct FailingActivationBackend;

    impl StateGraphConditionActivationBackend<Vec<u8>, ()> for FailingActivationBackend {
        type Error = &'static str;

        fn activation_func(
            &mut self,
            _request: ConditionActivationRequest<'_, Vec<u8>, ()>,
        ) -> Result<bool, Self::Error> {
            Err("activation failed")
        }
    }

    #[derive(Debug, Eq, PartialEq)]
    struct ActivationSource;

    impl fmt::Display for ActivationSource {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("activation source")
        }
    }

    impl Error for ActivationSource {}

    fn attribute(key: &[u8], text: &[u8], boolean_raw: u8) -> StateGraphBinaryAttribute {
        StateGraphBinaryAttribute {
            key: key.to_vec(),
            key_hash: state_graph_string_hash32(key),
            text: text.to_vec(),
            float_bits: 0,
            boolean_raw,
        }
    }

    fn element(
        name: &[u8],
        attributes: Vec<StateGraphBinaryAttribute>,
        children: Vec<StateGraphBinaryElement>,
    ) -> StateGraphBinaryElement {
        StateGraphBinaryElement {
            name: name.to_vec(),
            attributes,
            children,
        }
    }

    fn named_state(name: &[u8], children: Vec<StateGraphBinaryElement>) -> StateGraphBinaryElement {
        element(b"state", vec![attribute(b"name", name, 0)], children)
    }

    fn named_leaf(
        tag: &[u8],
        name: &[u8],
        children: Vec<StateGraphBinaryElement>,
    ) -> StateGraphBinaryElement {
        element(tag, vec![attribute(b"name", name, 0)], children)
    }

    fn expression(children: Vec<StateGraphBinaryElement>) -> StateGraphBinaryElement {
        element(b"expression", Vec::new(), children)
    }

    fn operator_expression(
        operator: &[u8],
        children: Vec<StateGraphBinaryElement>,
    ) -> StateGraphBinaryElement {
        element(b"expression", vec![attribute(b"op", operator, 0)], children)
    }

    fn condition(name: &[u8], mask: Option<&[u8]>, enabled: bool) -> StateGraphBinaryElement {
        let mut attributes = vec![attribute(b"name", name, 0)];
        if let Some(mask) = mask {
            attributes.push(attribute(b"mask", mask, 0));
        }
        if !enabled {
            attributes.push(attribute(b"enabled", b"false", 0));
        }
        element(b"condition", attributes, Vec::new())
    }

    fn activation_factory() -> TestFactory {
        TestFactory {
            conditions: [
                b"True".as_slice(),
                b"False",
                b"Later",
                b"RootTrue",
                b"RootFalse",
                b"ChildTrue",
                b"ChildFalse",
            ]
            .into_iter()
            .map(<[u8]>::to_vec)
            .collect(),
            ..TestFactory::default()
        }
    }

    fn activate(
        graph: &mut StateGraphRuntime<Vec<u8>>,
        expression: ExpressionId,
        mask: ConditionMask,
        backend: &mut TestActivationBackend,
    ) -> MaskedActivation {
        graph
            .evaluate_expression_masked(expression, mask, &(), backend)
            .unwrap()
    }

    #[test]
    fn construction_is_preorder_attaches_immediately_and_process_groups_by_native_vectors() {
        let behaviour = named_leaf(
            b"behaviour",
            b"bEh",
            vec![element(
                b"param",
                vec![attribute(b"value", b"x", 0)],
                Vec::new(),
            )],
        );
        let child_state = named_state(b"Child", Vec::new());
        let transition = element(
            b"transition",
            vec![attribute(b"target", b"Child", 0)],
            vec![
                expression(vec![named_leaf(b"condition", b"COND", Vec::new())]),
                named_leaf(b"hook", b"hook", Vec::new()),
            ],
        );
        let root_expression = expression(vec![named_leaf(b"condition", b"Cond", Vec::new())]);
        let root_element = named_state(
            b"Root",
            vec![behaviour, child_state, transition, root_expression],
        );
        let mut factory = TestFactory::shipped_slice();
        let mut graph =
            StateGraphRuntime::build(&root_element, &mut factory, FactoryMissPolicy::Strict)
                .unwrap();

        let root = graph.root().unwrap();
        assert_eq!(graph.states[root.0].children, [StateId(1)]);
        assert_eq!(graph.states[root.0].behaviours, [LeafId(0)]);
        assert_eq!(graph.states[root.0].transitions, [TransitionId(0)]);
        assert_eq!(graph.states[root.0].expression, Some(ExpressionId(1)));
        assert_eq!(graph.leaves[0].registered_name, b"Beh");
        assert_eq!(graph.leaves[0].base.path, b"Root.bEh");
        assert_eq!(graph.transitions[0].base.name, b"__unknown__");
        assert_eq!(graph.transitions[0].base.path, b"Root.__unknown__");
        assert_eq!(graph.params.len(), 1);
        assert!(graph.params[0].handled_by_override);
        assert_eq!(factory.params, [LeafId(0)]);

        let pushes = graph
            .construction_events()
            .iter()
            .filter(|event| matches!(event, ConstructionEvent::Push { .. }))
            .count();
        let pops = graph
            .construction_events()
            .iter()
            .filter(|event| matches!(event, ConstructionEvent::Pop { .. }))
            .count();
        assert_eq!((pushes, pops), (10, 10));
        let behaviour_attach = graph
            .construction_events()
            .iter()
            .position(|event| {
                *event
                    == ConstructionEvent::Attached {
                        parent: BaseId::State(root),
                        child: BaseId::Leaf(LeafId(0)),
                    }
            })
            .unwrap();
        let behaviour_complete = graph
            .construction_events()
            .iter()
            .position(|event| {
                *event
                    == ConstructionEvent::FactoryCreated {
                        kind: LeafKind::Behaviour,
                        leaf: LeafId(0),
                    }
            })
            .unwrap();
        assert!(behaviour_attach < behaviour_complete);

        let events = graph.process_after_build(&mut factory).unwrap();
        assert_eq!(
            events,
            [
                ProcessEvent::State(StateId(0)),
                ProcessEvent::Expression(ExpressionId(1)),
                ProcessEvent::Leaf(LeafId(3)),
                ProcessEvent::State(StateId(1)),
                ProcessEvent::Leaf(LeafId(0)),
                ProcessEvent::Transition(TransitionId(0)),
                ProcessEvent::Expression(ExpressionId(0)),
                ProcessEvent::Leaf(LeafId(1)),
                ProcessEvent::Leaf(LeafId(2)),
            ]
        );
        assert_eq!(
            factory.processed,
            [LeafId(3), LeafId(0), LeafId(1), LeafId(2)]
        );
    }

    #[test]
    fn empty_stack_uses_parser_parent_but_null_top_does_not_fall_through() {
        let mut factory = TestFactory::shipped_slice();
        let mut graph = StateGraphRuntime::build(
            &named_state(b"Parent", Vec::new()),
            &mut factory,
            FactoryMissPolicy::Strict,
        )
        .unwrap();
        graph
            .append(
                &named_state(b"Included", Vec::new()),
                Some(StateId(0)),
                &mut factory,
                FactoryMissPolicy::Strict,
            )
            .unwrap();
        assert_eq!(graph.states[1].parent_state, Some(StateId(0)));
        assert_eq!(graph.states[1].depth, 1);

        graph
            .append(
                &element(
                    b"unknown",
                    Vec::new(),
                    vec![named_state(b"Orphan", Vec::new())],
                ),
                Some(StateId(0)),
                &mut factory,
                FactoryMissPolicy::Strict,
            )
            .unwrap();
        assert_eq!(graph.states[2].parent_state, None);
        assert_eq!(graph.states[2].base.path, b"Orphan");

        let error = graph
            .append(
                &element(
                    b"unknown",
                    Vec::new(),
                    vec![element(
                        b"transition",
                        vec![attribute(b"target", b"Parent", 0)],
                        Vec::new(),
                    )],
                ),
                Some(StateId(0)),
                &mut factory,
                FactoryMissPolicy::Strict,
            )
            .unwrap_err();
        assert!(matches!(
            error,
            StateGraphRuntimeError::ParentAssertion { .. }
        ));
        let events = graph.process_after_build(&mut factory).unwrap();
        assert!(events.contains(&ProcessEvent::State(StateId(0))));
        assert!(events.contains(&ProcessEvent::State(StateId(1))));
        assert!(!events.contains(&ProcessEvent::State(StateId(2))));
    }

    #[test]
    fn param_pushes_null_so_its_child_state_is_parentless_and_base_param_errors() {
        let param = element(
            b"param",
            Vec::new(),
            vec![named_state(b"ParamChild", Vec::new())],
        );
        let root = named_state(b"Root", vec![named_leaf(b"behaviour", b"Beh", vec![param])]);
        let mut factory = TestFactory::shipped_slice();
        let graph =
            StateGraphRuntime::build(&root, &mut factory, FactoryMissPolicy::Strict).unwrap();
        assert_eq!(graph.states.len(), 2);
        assert_eq!(graph.states[1].parent_state, None);
        assert!(
            graph
                .construction_events()
                .iter()
                .any(|event| matches!(event, ConstructionEvent::Push { value: None, .. }))
        );

        let direct = named_state(b"Bad", vec![element(b"param", Vec::new(), Vec::new())]);
        let error =
            StateGraphRuntime::<Vec<u8>>::build(&direct, &mut factory, FactoryMissPolicy::Strict)
                .unwrap_err();
        assert_eq!(
            error.to_string(),
            "StateGraph Base::AddParam assertion for State(StateId(0))"
        );
    }

    #[test]
    fn second_node_expression_replaces_first_and_direct_fields_are_exact() {
        assert_eq!(TransitionPriority::Low as i32, 0);
        assert_eq!(TransitionPriority::Medium as i32, 1);
        assert_eq!(TransitionPriority::High as i32, 2);
        assert_eq!(TransitionPriority::Urgent as i32, 3);
        assert_eq!(ExpressionOperator::None as i32, 0);
        assert_eq!(ExpressionOperator::And as i32, 1);
        assert_eq!(ExpressionOperator::Or as i32, 2);
        assert_eq!(ExpressionOperator::Not as i32, 3);
        assert_eq!(ConditionMask::PreCond as i32, 1);
        assert_eq!(ConditionMask::Sustain as i32, 2);
        assert_eq!(ConditionMask::Always as i32, 3);
        assert_eq!(ConditionMask::PostCond as i32, 4);
        let first = element(b"expression", vec![attribute(b"op", b"and", 0)], Vec::new());
        let second = element(b"expression", vec![attribute(b"op", b"not", 0)], Vec::new());
        let child = element(
            b"state",
            vec![
                attribute(b"name", b"Child", 0),
                attribute(b"enabled", b"true", 1),
                attribute(b"active", b"false", 0),
                attribute(b"interruptable", b"Root", 0),
            ],
            vec![first, second],
        );
        let root = element(
            b"state",
            vec![
                attribute(b"name", b"Root", 0),
                attribute(b"enabled", b"false", 0),
                attribute(b"interruptable", b"false", 0),
            ],
            vec![child],
        );
        let mut factory = TestFactory::default();
        let mut graph =
            StateGraphRuntime::build(&root, &mut factory, FactoryMissPolicy::Strict).unwrap();
        assert!(!graph.states[0].base.enabled);
        assert_eq!(
            graph.states[0].interruptable,
            Interruptable::NotInterruptable
        );
        assert!(!graph.states[1].base.enabled);
        assert!(!graph.states[1].active);
        assert_eq!(graph.states[1].depth, 1);
        assert_eq!(graph.states[1].expression, Some(ExpressionId(1)));
        assert_eq!(graph.expressions[0].operator, ExpressionOperator::And);
        assert_eq!(graph.expressions[1].operator, ExpressionOperator::Not);
        assert!(graph.construction_events().iter().any(|event| matches!(
            event,
            ConstructionEvent::ReplacedExpression {
                previous: ExpressionId(0),
                replacement: ExpressionId(1),
                ..
            }
        )));
        let events = graph.process_after_build(&mut factory).unwrap();
        assert!(!events.contains(&ProcessEvent::Expression(ExpressionId(0))));
        assert!(events.contains(&ProcessEvent::Expression(ExpressionId(1))));
    }

    #[test]
    fn factory_miss_policy_is_strict_or_native_safe_null() {
        let root = named_state(
            b"Root",
            vec![expression(vec![named_leaf(
                b"condition",
                b"Missing",
                Vec::new(),
            )])],
        );
        let mut factory = TestFactory::default();
        let error =
            StateGraphRuntime::<Vec<u8>>::build(&root, &mut factory, FactoryMissPolicy::Strict)
                .unwrap_err();
        assert!(matches!(
            error,
            StateGraphRuntimeError::FactoryMiss {
                kind: LeafKind::Condition,
                ..
            }
        ));

        let graph =
            StateGraphRuntime::build(&root, &mut factory, FactoryMissPolicy::NativeSafeNull)
                .unwrap();
        assert!(graph.leaves.is_empty());
        assert!(graph.expressions[0].children.is_empty());
        assert!(graph.construction_events.iter().any(|event| matches!(
            event,
            ConstructionEvent::FactoryMiss {
                kind: LeafKind::Condition,
                ..
            }
        )));
        assert!(
            graph
                .construction_events
                .iter()
                .any(|event| matches!(event, ConstructionEvent::Push { value: None, .. }))
        );
    }

    #[test]
    fn find_and_process_resolve_dotted_targets_and_permanently_disable_transition() {
        let transition = element(
            b"transition",
            vec![
                attribute(b"target", b"Root.C.D", 0),
                attribute(b"priority", b"urgent", 0),
            ],
            Vec::new(),
        );
        let b = element(
            b"state",
            vec![
                attribute(b"name", b"B", 0),
                attribute(b"interruptable", b"A", 0),
            ],
            vec![transition],
        );
        let a = named_state(b"A", vec![b]);
        let c = element(
            b"state",
            vec![
                attribute(b"name", b"C", 0),
                attribute(b"enabled", b"false", 0),
            ],
            vec![named_state(b"D", Vec::new())],
        );
        let root = named_state(b"Root", vec![a, c]);
        let mut factory = TestFactory::default();
        let mut graph =
            StateGraphRuntime::build(&root, &mut factory, FactoryMissPolicy::Strict).unwrap();
        assert_eq!(
            graph.find_state(StateId(2), b"A", true).unwrap(),
            Some(StateId(1))
        );
        assert_eq!(
            graph.find_state(StateId(2), b"Root.C.D", true).unwrap(),
            Some(StateId(4))
        );
        assert_eq!(graph.transitions[0].priority, TransitionPriority::Urgent);
        graph.process_after_build(&mut factory).unwrap();
        assert_eq!(graph.transitions[0].target, Some(StateId(4)));
        assert!(!graph.transitions[0].base.enabled);
        assert!(matches!(
            graph.states[2].interruptable,
            Interruptable::StateInterruptable {
                target: Some(StateId(1)),
                ..
            }
        ));
        assert!(matches!(
            graph.process_after_build(&mut factory),
            Err(StateGraphRuntimeError::AlreadyProcessed)
        ));
    }

    #[test]
    fn invalid_interrupt_sibling_is_rejected_after_find() {
        let a = element(
            b"state",
            vec![
                attribute(b"name", b"A", 0),
                attribute(b"interruptable", b"C", 0),
            ],
            Vec::new(),
        );
        let mut graph_root = named_state(b"Root", vec![a, named_state(b"C", Vec::new())]);
        // Keep the owned tree visibly mutable to exercise the public decoded type.
        graph_root.name = b"state".to_vec();
        let mut factory = TestFactory::default();
        let mut graph =
            StateGraphRuntime::build(&graph_root, &mut factory, FactoryMissPolicy::Strict).unwrap();
        assert!(matches!(
            graph.process_after_build(&mut factory),
            Err(StateGraphRuntimeError::InvalidInterruptTarget {
                state: StateId(1),
                ..
            })
        ));
    }

    #[test]
    fn public_decoded_boundary_checks_wrapper_include_attributes_hashes_nuls_and_stack() {
        let mut factory = TestFactory::default();
        assert!(matches!(
            StateGraphRuntime::<Vec<u8>>::build(
                &element(b"stategraph", Vec::new(), Vec::new()),
                &mut factory,
                FactoryMissPolicy::Strict,
            ),
            Err(StateGraphRuntimeError::CompiledStateGraphWrapper)
        ));
        let include = named_state(
            b"Root",
            vec![element(
                b"include",
                vec![attribute(b"filename", b"child.xml", 0)],
                Vec::new(),
            )],
        );
        assert!(matches!(
            StateGraphRuntime::<Vec<u8>>::build(
                &include,
                &mut factory,
                FactoryMissPolicy::Strict,
            ),
            Err(StateGraphRuntimeError::UnsupportedInclude(name)) if name == b"child.xml"
        ));

        let mut bad_hash = named_state(b"Root", Vec::new());
        bad_hash.attributes[0].key_hash ^= 1;
        assert!(matches!(
            StateGraphRuntime::<Vec<u8>>::build(&bad_hash, &mut factory, FactoryMissPolicy::Strict,),
            Err(StateGraphRuntimeError::InvalidAttributeHash { .. })
        ));
        let nul = element(b"sta\0te", Vec::new(), Vec::new());
        assert!(matches!(
            StateGraphRuntime::<Vec<u8>>::build(&nul, &mut factory, FactoryMissPolicy::Strict,),
            Err(StateGraphRuntimeError::EmbeddedNul { .. })
        ));

        let mut nested = named_state(b"Leaf", Vec::new());
        for index in 0..24 {
            nested = named_state(format!("S{index}").as_bytes(), vec![nested]);
        }
        assert!(matches!(
            StateGraphRuntime::<Vec<u8>>::build(&nested, &mut factory, FactoryMissPolicy::Strict,),
            Err(StateGraphRuntimeError::StackCapacity { maximum: 24 })
        ));
    }

    #[test]
    fn exact_operator_mask_and_priority_cases_keep_native_defaults() {
        let transition = element(
            b"transition",
            vec![
                attribute(b"target", b"Root", 0),
                attribute(b"priority", b"MED", 0),
            ],
            vec![expression(vec![element(
                b"condition",
                vec![
                    attribute(b"name", b"Cond", 0),
                    attribute(b"mask", b"postcond", 0),
                ],
                Vec::new(),
            )])],
        );
        let root = named_state(b"Root", vec![transition]);
        let mut factory = TestFactory::shipped_slice();
        let graph =
            StateGraphRuntime::build(&root, &mut factory, FactoryMissPolicy::Strict).unwrap();
        assert_eq!(graph.transitions[0].priority, TransitionPriority::Low);
        assert_eq!(graph.expressions[0].operator, ExpressionOperator::None);
        assert_eq!(graph.leaves[0].mask, Some(ConditionMask::PostCond));

        let invalid_expression = named_state(
            b"Root",
            vec![element(
                b"expression",
                vec![attribute(b"op", b"AND", 0)],
                Vec::new(),
            )],
        );
        assert!(matches!(
            StateGraphRuntime::<Vec<u8>>::build(
                &invalid_expression,
                &mut factory,
                FactoryMissPolicy::Strict,
            ),
            Err(StateGraphRuntimeError::InvalidOperator(value)) if value == b"AND"
        ));
    }

    #[test]
    fn activation_masks_use_exact_bits_and_masked_conditions_do_not_call_backend() {
        let root = named_state(
            b"Root",
            vec![
                named_state(
                    b"Pre",
                    vec![expression(vec![condition(
                        b"False",
                        Some(b"precond"),
                        true,
                    )])],
                ),
                named_state(
                    b"Sustain",
                    vec![expression(vec![condition(
                        b"False",
                        Some(b"sustain"),
                        true,
                    )])],
                ),
                named_state(
                    b"Always",
                    vec![expression(vec![condition(b"False", Some(b"always"), true)])],
                ),
                named_state(
                    b"Post",
                    vec![expression(vec![condition(
                        b"False",
                        Some(b"postcond"),
                        true,
                    )])],
                ),
                named_state(
                    b"Disabled",
                    vec![expression(vec![condition(b"True", Some(b"always"), false)])],
                ),
            ],
        );
        let mut factory = activation_factory();
        let mut graph =
            StateGraphRuntime::build(&root, &mut factory, FactoryMissPolicy::Strict).unwrap();
        let expressions = (1..=5)
            .map(|state| graph.states[state].expression.unwrap())
            .collect::<Vec<_>>();
        let mut backend = TestActivationBackend::default();

        let requested_masks = [
            ConditionMask::PreCond,
            ConditionMask::Sustain,
            ConditionMask::Always,
            ConditionMask::PostCond,
        ];
        let intersects = [
            [true, false, true, false],
            [false, true, true, false],
            [true, true, true, false],
            [false, false, false, true],
        ];
        for (stored_index, row) in intersects.into_iter().enumerate() {
            for (requested, should_call) in requested_masks.into_iter().zip(row) {
                let result = graph
                    .evaluate_expression_masked(
                        expressions[stored_index],
                        requested,
                        &(),
                        &mut backend,
                    )
                    .unwrap();
                assert_eq!(
                    result,
                    MaskedActivation {
                        value: !should_call,
                        was_masked: !should_call,
                    },
                    "stored row {stored_index}, requested {requested:?}"
                );
                if should_call {
                    assert_eq!(backend.calls, [LeafId(stored_index)]);
                } else {
                    assert!(backend.calls.is_empty());
                }
                backend.calls.clear();
            }
        }

        for requested in requested_masks {
            assert_eq!(
                graph
                    .evaluate_expression_masked(expressions[4], requested, &(), &mut backend,)
                    .unwrap(),
                MaskedActivation {
                    value: true,
                    was_masked: true,
                }
            );
            assert!(backend.calls.is_empty());
        }
    }

    #[test]
    fn expression_activation_preserves_empty_and_all_masked_identities() {
        let root = named_state(
            b"Root",
            vec![
                named_state(b"EmptyNone", vec![expression(Vec::new())]),
                named_state(b"EmptyAnd", vec![operator_expression(b"and", Vec::new())]),
                named_state(b"EmptyOr", vec![operator_expression(b"or", Vec::new())]),
                named_state(
                    b"MaskedAnd",
                    vec![operator_expression(
                        b"and",
                        vec![condition(b"True", Some(b"sustain"), true)],
                    )],
                ),
                named_state(
                    b"MaskedOr",
                    vec![operator_expression(
                        b"or",
                        vec![condition(b"True", Some(b"sustain"), true)],
                    )],
                ),
                named_state(
                    b"MaskedNot",
                    vec![operator_expression(
                        b"not",
                        vec![condition(b"True", Some(b"sustain"), true)],
                    )],
                ),
            ],
        );
        let mut factory = activation_factory();
        let mut graph =
            StateGraphRuntime::build(&root, &mut factory, FactoryMissPolicy::Strict).unwrap();
        let expressions = (1..=6)
            .map(|state| graph.states[state].expression.unwrap())
            .collect::<Vec<_>>();
        let mut backend = TestActivationBackend::default();

        for expression in &expressions[0..6] {
            assert_eq!(
                graph
                    .evaluate_expression_masked(
                        *expression,
                        ConditionMask::PreCond,
                        &(),
                        &mut backend,
                    )
                    .unwrap(),
                MaskedActivation {
                    value: true,
                    was_masked: true,
                }
            );
        }
        assert!(backend.calls.is_empty());
    }

    #[test]
    fn expression_activation_preserves_short_circuit_and_unmasked_order() {
        let root = named_state(
            b"Root",
            vec![
                named_state(
                    b"ShortAnd",
                    vec![operator_expression(
                        b"and",
                        vec![
                            condition(b"True", None, true),
                            condition(b"False", None, true),
                            condition(b"Later", None, true),
                        ],
                    )],
                ),
                named_state(
                    b"ShortOr",
                    vec![operator_expression(
                        b"or",
                        vec![
                            condition(b"False", None, true),
                            condition(b"True", None, true),
                            condition(b"Later", None, true),
                        ],
                    )],
                ),
                named_state(
                    b"MixedAnd",
                    vec![operator_expression(
                        b"and",
                        vec![
                            condition(b"True", Some(b"sustain"), true),
                            condition(b"False", None, true),
                        ],
                    )],
                ),
                named_state(
                    b"NotFalse",
                    vec![operator_expression(
                        b"not",
                        vec![condition(b"False", None, true)],
                    )],
                ),
            ],
        );
        let mut factory = activation_factory();
        let mut graph =
            StateGraphRuntime::build(&root, &mut factory, FactoryMissPolicy::Strict).unwrap();
        let expressions = (1..=4)
            .map(|state| graph.states[state].expression.unwrap())
            .collect::<Vec<_>>();
        let mut backend = TestActivationBackend::default();

        assert!(
            !activate(
                &mut graph,
                expressions[0],
                ConditionMask::PreCond,
                &mut backend,
            )
            .value
        );
        assert_eq!(backend.calls, [LeafId(0), LeafId(1)]);
        backend.calls.clear();
        assert!(
            activate(
                &mut graph,
                expressions[1],
                ConditionMask::PreCond,
                &mut backend,
            )
            .value
        );
        assert_eq!(backend.calls, [LeafId(3), LeafId(4)]);
        backend.calls.clear();
        assert!(
            !activate(
                &mut graph,
                expressions[2],
                ConditionMask::PreCond,
                &mut backend,
            )
            .value
        );
        assert_eq!(backend.calls, [LeafId(7)]);
        backend.calls.clear();
        assert!(
            activate(
                &mut graph,
                expressions[3],
                ConditionMask::PreCond,
                &mut backend,
            )
            .value
        );
        assert_eq!(backend.calls, [LeafId(8)]);
    }

    #[test]
    fn expression_activation_recurses_and_short_circuits_nested_operators() {
        let root = named_state(
            b"Root",
            vec![operator_expression(
                b"and",
                vec![
                    condition(b"True", None, true),
                    operator_expression(
                        b"or",
                        vec![
                            condition(b"False", None, true),
                            condition(b"True", None, true),
                            condition(b"Later", None, true),
                        ],
                    ),
                    condition(b"False", None, true),
                ],
            )],
        );
        let mut factory = activation_factory();
        let mut graph =
            StateGraphRuntime::build(&root, &mut factory, FactoryMissPolicy::Strict).unwrap();
        let expression = graph.states[0].expression.unwrap();
        let mut backend = TestActivationBackend::default();

        assert!(!activate(&mut graph, expression, ConditionMask::PreCond, &mut backend,).value);
        assert_eq!(backend.calls, [LeafId(0), LeafId(1), LeafId(2), LeafId(4)]);
    }

    #[test]
    fn activation_propagates_concrete_backend_errors() {
        let root = named_state(
            b"Root",
            vec![expression(vec![condition(b"True", None, true)])],
        );
        let mut factory = activation_factory();
        let mut graph =
            StateGraphRuntime::build(&root, &mut factory, FactoryMissPolicy::Strict).unwrap();
        let expression = graph.states[0].expression.unwrap();
        let mut backend = FailingActivationBackend;

        assert!(matches!(
            graph
                .evaluate_expression_masked(expression, ConditionMask::PreCond, &(), &mut backend,),
            Err(StateGraphActivationError::Backend("activation failed"))
        ));
    }

    #[test]
    fn activation_backend_error_preserves_standard_source_chain() {
        let error = StateGraphActivationError::Backend(ActivationSource);
        assert_eq!(
            Error::source(&error).and_then(|source| source.downcast_ref::<ActivationSource>()),
            Some(&ActivationSource)
        );
        assert!(
            Error::source(&StateGraphActivationError::<ActivationSource>::InvalidState(StateId(0)))
                .is_none()
        );
    }

    #[test]
    fn node_and_ancestor_activation_use_self_first_and_treat_missing_expression_as_true() {
        let root = named_state(
            b"Root",
            vec![
                expression(vec![condition(b"RootTrue", None, true)]),
                named_state(
                    b"ChildFalse",
                    vec![expression(vec![condition(b"ChildFalse", None, true)])],
                ),
                named_state(
                    b"ChildTrue",
                    vec![expression(vec![condition(b"ChildTrue", None, true)])],
                ),
                named_state(b"NoExpression", Vec::new()),
                element(
                    b"transition",
                    vec![attribute(b"target", b"Root", 0)],
                    Vec::new(),
                ),
                element(
                    b"transition",
                    vec![attribute(b"target", b"Root", 0)],
                    vec![expression(vec![condition(b"False", None, true)])],
                ),
            ],
        );
        let mut factory = activation_factory();
        let mut graph =
            StateGraphRuntime::build(&root, &mut factory, FactoryMissPolicy::Strict).unwrap();
        let mut backend = TestActivationBackend::default();

        assert!(
            !graph
                .state_activation_parent(StateId(1), ConditionMask::PreCond, &(), &mut backend,)
                .unwrap()
        );
        assert_eq!(backend.calls, [LeafId(1)]);
        backend.calls.clear();
        assert!(
            graph
                .state_activation_parent(StateId(2), ConditionMask::PreCond, &(), &mut backend,)
                .unwrap()
        );
        assert_eq!(backend.calls, [LeafId(2), LeafId(0)]);
        backend.calls.clear();
        assert!(
            graph
                .state_activation(StateId(3), ConditionMask::PreCond, &(), &mut backend,)
                .unwrap()
        );
        assert!(
            graph
                .transition_activation(TransitionId(0), ConditionMask::PreCond, &(), &mut backend,)
                .unwrap()
        );
        assert!(
            !graph
                .transition_activation(TransitionId(1), ConditionMask::PreCond, &(), &mut backend,)
                .unwrap()
        );
        assert_eq!(backend.calls, [LeafId(3)]);
    }

    #[test]
    fn activation_rejects_invalid_ids_kinds_masks_payloads_and_not_arity() {
        let root = named_state(
            b"Root",
            vec![expression(vec![condition(b"True", None, true)])],
        );
        let mut factory = activation_factory();
        let mut graph =
            StateGraphRuntime::build(&root, &mut factory, FactoryMissPolicy::Strict).unwrap();
        let mut backend = TestActivationBackend::default();
        let expression = graph.states[0].expression.unwrap();

        assert!(matches!(
            graph.evaluate_expression_masked(
                ExpressionId(99),
                ConditionMask::PreCond,
                &(),
                &mut backend,
            ),
            Err(StateGraphActivationError::InvalidExpression(ExpressionId(
                99
            )))
        ));
        assert!(matches!(
            graph.state_activation(StateId(99), ConditionMask::PreCond, &(), &mut backend,),
            Err(StateGraphActivationError::InvalidState(StateId(99)))
        ));
        assert!(matches!(
            graph.transition_activation(
                TransitionId(99),
                ConditionMask::PreCond,
                &(),
                &mut backend,
            ),
            Err(StateGraphActivationError::InvalidTransition(TransitionId(
                99
            )))
        ));

        graph.expressions[expression.0].children[0] = ExpressionChildId::Condition(LeafId(99));
        assert!(matches!(
            graph
                .evaluate_expression_masked(expression, ConditionMask::PreCond, &(), &mut backend,),
            Err(StateGraphActivationError::InvalidLeaf(LeafId(99)))
        ));
        graph.expressions[expression.0].children[0] = ExpressionChildId::Condition(LeafId(0));

        graph.leaves[0].kind = LeafKind::Behaviour;
        assert!(matches!(
            graph
                .evaluate_expression_masked(expression, ConditionMask::PreCond, &(), &mut backend,),
            Err(StateGraphActivationError::NonConditionLeaf {
                leaf: LeafId(0),
                actual: LeafKind::Behaviour,
            })
        ));
        graph.leaves[0].kind = LeafKind::Condition;
        graph.leaves[0].mask = None;
        assert!(matches!(
            graph
                .evaluate_expression_masked(expression, ConditionMask::PreCond, &(), &mut backend,),
            Err(StateGraphActivationError::MissingConditionMask(LeafId(0)))
        ));
        graph.leaves[0].mask = Some(ConditionMask::PreCond);
        graph.leaves[0].payload = None;
        assert!(matches!(
            graph
                .evaluate_expression_masked(expression, ConditionMask::PreCond, &(), &mut backend,),
            Err(StateGraphActivationError::UninitializedCondition(LeafId(0)))
        ));

        graph.expressions[expression.0].operator = ExpressionOperator::Not;
        graph.expressions[expression.0].children.clear();
        assert!(matches!(
            graph.evaluate_expression_masked(
                expression,
                ConditionMask::PreCond,
                &(),
                &mut backend,
            ),
            Err(StateGraphActivationError::InvalidNotArity {
                expression: bad,
                children: 0,
            }) if bad == expression
        ));
        graph.expressions[expression.0].children = vec![
            ExpressionChildId::Condition(LeafId(0)),
            ExpressionChildId::Condition(LeafId(0)),
        ];
        assert!(matches!(
            graph
                .evaluate_expression_masked(expression, ConditionMask::PreCond, &(), &mut backend,),
            Err(StateGraphActivationError::InvalidNotArity { children: 2, .. })
        ));
    }
}
