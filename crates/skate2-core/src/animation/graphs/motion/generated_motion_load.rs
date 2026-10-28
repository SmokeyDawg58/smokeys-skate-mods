//! Generated Motion declarations and retained eager factory load.
//! Constructor8292a1b0 must finish all factories before an executable controller
//! may be published. Unsupported constructors never become successful objects.
#![expect(
    dead_code,
    reason = "source constructor fields are retained for Motion callbacks and controller Update, whose consumers are not yet ported"
)]
use crate::generated_action_load::{
    Attribute, BehaviourDeclaration, ConditionDeclaration, Section, SourceBinding,
    StateDeclaration, TransitionDeclaration,
};
use std::sync::{Arc, Mutex};

#[path = "generated_motion_load_conditions.rs"]
pub(crate) mod conditions;
#[path = "generated_motion_load_construct.rs"]
pub(crate) mod construct;
#[path = "generated_motion_load_controller.rs"]
pub(crate) mod controller;
#[path = "generated_motion_load_data.rs"]
pub(crate) mod data;
#[path = "generated_motion_load_hooks.rs"]
pub(crate) mod hooks;
#[path = "generated_motion_ids.rs"]
pub(crate) mod ids;
#[path = "generated_motion_runtime.rs"]
pub(crate) mod runtime;
#[cfg(test)]
#[path = "generated_motion_load_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HookDeclaration {
    pub(crate) name: &'static str,
    pub(crate) attributes: Section,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ObjectDomain {
    Behaviour,
    Hook,
    Condition,
}
#[derive(Debug)]
pub(crate) enum MotionLoadError {
    ContextRequired,
    ForeignContext,
    Allocation,
    MissingCvars,
    Cvar(crate::input_cvar_runtime::InputCvarError),
    AttributeRuntime(crate::attribute_runtime::AttributeRuntimeError),
    MissingVault,
    Lock,
    AlreadyAttempted,
    DescriptorBounds,
    IncompleteConstructor {
        domain: ObjectDomain,
        index: usize,
        name: &'static str,
    },
    InvalidAttribute {
        key: &'static str,
        value: String,
    },
    PackedString(&'static str),
    UnsupportedParameters(usize),
    PlayAnimation(crate::play_animation::PlayAnimationParseError),
    NullParameterTarget(usize),
    Shared(crate::generated_action_load::GeneratedActionLoadError),
}
impl std::fmt::Display for MotionLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "generated Motion load: {self:?}")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MotionAllocation {
    Host,
    Null,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct AllocationSite {
    pub(crate) domain: ObjectDomain,
    pub(crate) index: usize,
    pub(crate) binding: SourceBinding,
}
struct StaticState {
    vault: Option<Arc<crate::attribute_runtime::AttributeRuntimeVault>>,
    printed: bool,
    behaviours_initialized: bool,
    conditions_initialized: bool,
    conditions: Vec<Option<Arc<conditions::MotionCondition>>>,
    cvars: Option<crate::input_cvar_runtime::InputCvarReferences>,
    behaviours: Vec<Option<Arc<construct::MotionBehaviour>>>,
    names: std::collections::BTreeMap<&'static str, Arc<str>>,
}
#[derive(Clone)]
pub(crate) struct GeneratedMotionStatics {
    inner: Arc<Mutex<StaticState>>,
}
impl GeneratedMotionStatics {
    pub(crate) fn new(
        vault: Option<Arc<crate::attribute_runtime::AttributeRuntimeVault>>,
        cvars: Option<crate::input_cvar_runtime::InputCvarReferences>,
    ) -> Self {
        Self {
            inner: Arc::new(Mutex::new(StaticState {
                vault,
                printed: false,
                behaviours_initialized: false,
                conditions_initialized: false,
                conditions: (0..data::CONDITIONS.len()).map(|_| None).collect(),
                cvars,
                behaviours: (0..data::BEHAVIOURS.len()).map(|_| None).collect(),
                names: std::collections::BTreeMap::new(),
            })),
        }
    }
    pub(crate) fn behaviour(
        &self,
        index: usize,
    ) -> Result<Option<Arc<construct::MotionBehaviour>>, MotionLoadError> {
        self.inner
            .lock()
            .map_err(|_| MotionLoadError::Lock)?
            .behaviours
            .get(index)
            .cloned()
            .ok_or(MotionLoadError::DescriptorBounds)
    }
}
/// Retained factory allocations, never an executable graph controller by itself.
/// Same shared statics survive a failed constructor exactly as source stores do.
pub(crate) struct GeneratedMotionLoad {
    statics: GeneratedMotionStatics,
    attempted: bool,
    hooks: Vec<Option<Arc<hooks::MotionHook>>>,
}
impl GeneratedMotionLoad {
    pub(crate) fn new(statics: &GeneratedMotionStatics) -> Self {
        Self {
            statics: statics.clone(),
            attempted: false,
            hooks: (0..data::HOOKS.len()).map(|_| None).collect(),
        }
    }
    /// Source publishes each pointer before visiting its ordered `AddParam` list;
    /// sInit is written after each completed iteration, including NULL results.
    #[expect(
        clippy::too_many_lines,
        reason = "source eager load phases and publication order remain visible together"
    )]
    pub(crate) fn try_construct(
        &mut self,
        allocate: &mut impl FnMut(AllocationSite) -> Result<MotionAllocation, MotionLoadError>,
    ) -> Result<(), MotionLoadError> {
        if self.attempted {
            return Err(MotionLoadError::AlreadyAttempted);
        }
        self.attempted = true;
        let mut state = self
            .statics
            .inner
            .lock()
            .map_err(|_| MotionLoadError::Lock)?;
        state.printed = true;
        if !state.behaviours_initialized {
            for (index, declaration) in data::BEHAVIOURS.iter().enumerate() {
                let binding = construct::binding(declaration.name).ok_or(
                    MotionLoadError::IncompleteConstructor {
                        domain: ObjectDomain::Behaviour,
                        index,
                        name: declaration.name,
                    },
                )?;
                state.behaviours[index] = match allocate(AllocationSite {
                    domain: ObjectDomain::Behaviour,
                    index,
                    binding,
                })? {
                    MotionAllocation::Host => {
                        let name = Arc::clone(
                            state
                                .names
                                .entry(declaration.name)
                                .or_insert_with(|| Arc::from(declaration.name)),
                        );
                        Some(Arc::new(construct::behaviour_with_vault(
                            declaration,
                            name,
                            state.vault.as_ref(),
                        )?))
                    }
                    MotionAllocation::Null => None,
                };
                for parameter in 0..usize::from(declaration.parameters.count) {
                    let slot = usize::from(declaration.parameters.index) + parameter;
                    let parameter_index = usize::from(
                        *data::INDICES
                            .get(slot)
                            .ok_or(MotionLoadError::DescriptorBounds)?,
                    );
                    let section = *data::PARAMETERS
                        .get(parameter_index)
                        .ok_or(MotionLoadError::DescriptorBounds)?;
                    let object = state.behaviours[index]
                        .as_mut()
                        .ok_or(MotionLoadError::NullParameterTarget(index))?;
                    // The constructor has not returned; no callback or caller
                    // can acquire this just-published object during AddParam.
                    construct::add_parameter(
                        Arc::get_mut(object).ok_or(MotionLoadError::Lock)?,
                        section,
                        index,
                    )?;
                }
                state.behaviours_initialized = true;
            }
        }
        // Hooks belong to this controller and are eagerly visited in source order.
        for (index, declaration) in data::HOOKS.iter().enumerate() {
            let binding =
                hooks::binding(declaration.name).ok_or(MotionLoadError::IncompleteConstructor {
                    domain: ObjectDomain::Hook,
                    index,
                    name: declaration.name,
                })?;
            self.hooks[index] = match allocate(AllocationSite {
                domain: ObjectDomain::Hook,
                index,
                binding,
            })? {
                MotionAllocation::Host => {
                    let name = Arc::clone(
                        state
                            .names
                            .entry(declaration.name)
                            .or_insert_with(|| Arc::from(declaration.name)),
                    );
                    Some(Arc::new(hooks::construct(declaration, name)?))
                }
                MotionAllocation::Null => None,
            };
        }
        // Delegate::Init publishes each condition pointer, then sInit only after
        // the ENTIRE condition loop (unlike the behaviour loop's inner store).
        if !state.conditions_initialized {
            for (index, declaration) in data::CONDITIONS.iter().enumerate() {
                let binding = conditions::bindings::binding(declaration.name).ok_or(
                    MotionLoadError::IncompleteConstructor {
                        domain: ObjectDomain::Condition,
                        index,
                        name: declaration.name,
                    },
                )?;
                state.conditions[index] = match allocate(AllocationSite {
                    domain: ObjectDomain::Condition,
                    index,
                    binding,
                })? {
                    MotionAllocation::Null => None,
                    MotionAllocation::Host => {
                        let name = Arc::clone(
                            state
                                .names
                                .entry(declaration.name)
                                .or_insert_with(|| Arc::from(declaration.name)),
                        );
                        Some(Arc::new(conditions::construct(
                            declaration,
                            name,
                            &mut || {
                                state
                                    .cvars
                                    .as_ref()
                                    .ok_or(MotionLoadError::MissingCvars)?
                                    .try_mo_angle_max_delta()
                                    .map_err(MotionLoadError::Cvar)
                            },
                        )?))
                    }
                };
            }
            state.conditions_initialized = true;
        }
        Ok(())
    }
}
