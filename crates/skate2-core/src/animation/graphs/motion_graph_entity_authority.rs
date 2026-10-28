//! Opaque Actor-entity authority for bound `MotionGraph` callbacks.
//!
//! Native `ActionGraph` and `MotionGraph` wrappers retain the same non-owning
//! `Sk8::Entity*`, while each wrapper installs its own controller pointer in
//! its inline `StateGraph::Context`. That evidence motivates these host
//! handles. The current Motion-only session preserves one issuance across its
//! own calls; Actor-level cross-graph sharing remains deferred. No native
//! address, ABI layout, ownership, liveness, UCOM lookup, or thread semantic is
//! claimed.

use std::{fmt, sync::Arc};

use crate::{
    motion_graph_conditions::AnimProperty,
    play_animation::{FastString30, FastString36},
    play_animation_lifecycle::{
        AnimationDispatch, PhysicalOutputObservation, PlayAnimationInterfaceKind,
        PlayAnimationInterfaceResolution, RawAnimationTransitionParams, TreeAttributeSample,
    },
    play_remapped_animation::{
        PlayRemappedAnimationCollaborator, RemappedAnimationSetup, RemappedAnimationUpdate,
    },
    state_graph_context::{StateGraphContext, StateGraphContextHandle},
};

struct MotionGraphEntityAuthorityInner<K> {
    key: K,
}

/// One issued logical Actor-entity identity.
///
/// Equality compares issuance identity, not `K`. Two independently issued
/// authorities containing equal keys are deliberately different entities.
/// The handle has no public `Clone`; shallow copies are made only through the
/// explicit Context-handle contract.
///
/// ```compile_fail
/// use skate2_core::motion_graph_entity_authority::MotionGraphEntityAuthority;
/// let _forged = MotionGraphEntityAuthority::issue(7_u32);
/// ```
pub struct MotionGraphEntityAuthority<K> {
    inner: Arc<MotionGraphEntityAuthorityInner<K>>,
}

impl<K> MotionGraphEntityAuthority<K> {
    #[must_use]
    pub(crate) fn issue(key: K) -> Self {
        Self {
            inner: Arc::new(MotionGraphEntityAuthorityInner { key }),
        }
    }

    #[must_use]
    pub fn key(&self) -> &K {
        &self.inner.key
    }

    #[must_use]
    pub fn is_same_issuance(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl<K> StateGraphContextHandle for MotionGraphEntityAuthority<K> {
    fn shallow_copy(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<K> PartialEq for MotionGraphEntityAuthority<K> {
    fn eq(&self, other: &Self) -> bool {
        self.is_same_issuance(other)
    }
}

impl<K> Eq for MotionGraphEntityAuthority<K> {}

impl<K> fmt::Debug for MotionGraphEntityAuthority<K> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MotionGraphEntityAuthority")
            .finish_non_exhaustive()
    }
}

struct MotionGraphControllerTokenInner;

/// Logical identity of one `MotionGraph` controller wrapper.
///
/// This is intentionally not a pointer to the Rust controller or the native
/// wrapper address. Only the bound-session module can issue one.
///
/// ```compile_fail
/// use skate2_core::motion_graph_entity_authority::MotionGraphControllerToken;
/// let _forged = MotionGraphControllerToken::issue();
/// ```
pub struct MotionGraphControllerToken {
    inner: Arc<MotionGraphControllerTokenInner>,
}

impl MotionGraphControllerToken {
    pub(crate) fn issue() -> Self {
        Self {
            inner: Arc::new(MotionGraphControllerTokenInner),
        }
    }

    #[must_use]
    pub fn is_same_issuance(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl StateGraphContextHandle for MotionGraphControllerToken {
    fn shallow_copy(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl PartialEq for MotionGraphControllerToken {
    fn eq(&self, other: &Self) -> bool {
        self.is_same_issuance(other)
    }
}

impl Eq for MotionGraphControllerToken {}

impl fmt::Debug for MotionGraphControllerToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("MotionGraphControllerToken(..)")
    }
}

/// Opaque identity of one controller-owned generated Behaviour instance.
/// Payload storage and lifetime stay with the issuing controller.
pub struct MotionGraphInstanceToken {
    inner: Arc<MotionGraphInstanceTokenInner>,
}
struct MotionGraphInstanceTokenInner;
impl MotionGraphInstanceToken {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "generated Behaviour instance allocation is not yet mounted"
        )
    )]
    pub(crate) fn issue() -> Self {
        Self {
            inner: Arc::new(MotionGraphInstanceTokenInner),
        }
    }
    #[must_use]
    pub fn is_same_issuance(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}
impl StateGraphContextHandle for MotionGraphInstanceToken {
    fn shallow_copy(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}
impl PartialEq for MotionGraphInstanceToken {
    fn eq(&self, other: &Self) -> bool {
        self.is_same_issuance(other)
    }
}
impl Eq for MotionGraphInstanceToken {}
impl fmt::Debug for MotionGraphInstanceToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("MotionGraphInstanceToken(..)")
    }
}

/// Uninhabited semantic handle for a Context slot proven null in the current
/// constructor slice but not yet backed by a live host subsystem.
#[derive(Debug)]
pub enum NullStateGraphContextHandle {}

impl StateGraphContextHandle for NullStateGraphContextHandle {
    fn shallow_copy(&self) -> Self {
        match *self {}
    }
}

/// Exact six-slot Context shape used by the authority-bound dynamic session.
///
/// The three uninhabited slots keep `CameraMan`, `BehaviourSource`, and
/// `BehaviourSink` null. Generated callbacks may retain an Instance identity.
/// The active `PlayAnimation` instance remains a separate callback overlay.
pub type BoundMotionGraphContext<K> = StateGraphContext<
    MotionGraphControllerToken,
    MotionGraphEntityAuthority<K>,
    MotionGraphInstanceToken,
    NullStateGraphContextHandle,
    NullStateGraphContextHandle,
    NullStateGraphContextHandle,
>;

/// Sole public issuer for one logical Actor Entity authority.
///
/// Multiple graph facades created from one owner receive shallow handles to
/// the same issuance. The current code implements only the Motion facade;
/// `ActionGraph` construction and Actor-level destruction order remain future
/// boundaries.
pub struct ActorGraphAuthority<K> {
    entity: MotionGraphEntityAuthority<K>,
}

impl<K> ActorGraphAuthority<K> {
    #[must_use]
    pub fn new(entity_key: K) -> Self {
        Self {
            entity: MotionGraphEntityAuthority::issue(entity_key),
        }
    }

    #[must_use]
    pub fn entity_key(&self) -> &K {
        self.entity.key()
    }

    /// Retains this exact issuance for an owned component of the same Actor.
    /// This neither issues a new identity nor transfers Actor lifetime authority.
    pub(crate) fn entity_handle(&self) -> MotionGraphEntityAuthority<K> {
        self.entity.shallow_copy()
    }

    #[must_use]
    pub fn motion_graph_facade<R>(&self, runtime: R) -> MotionGraphRuntimeFacade<K, R> {
        MotionGraphRuntimeFacade::from_authority(self.entity.shallow_copy(), runtime)
    }
}

/// `HasIntent` services whose target authority is supplied on every call.
#[allow(clippy::missing_errors_doc)]
pub trait MotionGraphHasIntentRuntime<K> {
    type Error;
    type MotionGraph: PartialEq;
    type Intents;

    fn cached_motion_graph(
        &mut self,
        entity: &MotionGraphEntityAuthority<K>,
    ) -> Result<Option<Self::MotionGraph>, Self::Error>;
    fn find_motion_graph(
        &mut self,
        entity: &MotionGraphEntityAuthority<K>,
    ) -> Result<Option<Self::MotionGraph>, Self::Error>;
    fn motion_graph_intents(
        &mut self,
        entity: &MotionGraphEntityAuthority<K>,
        motion_graph: &Self::MotionGraph,
    ) -> Result<Self::Intents, Self::Error>;
    fn lookup_intent(
        &mut self,
        entity: &MotionGraphEntityAuthority<K>,
        intents: &Self::Intents,
        intent_name: &[u8],
    ) -> Result<Option<f32>, Self::Error>;
}

/// `WillExpire` services whose target authority is supplied on every call.
#[allow(clippy::missing_errors_doc)]
pub trait MotionGraphWillExpireRuntime<K> {
    type Error;
    type AnimInfo;
    type Animatable: PartialEq;

    fn find_anim_info(
        &mut self,
        entity: &MotionGraphEntityAuthority<K>,
    ) -> Result<Option<Self::AnimInfo>, Self::Error>;
    fn resolve_special_tag(
        &mut self,
        entity: &MotionGraphEntityAuthority<K>,
        anim_info: &Self::AnimInfo,
        tag: &[u8],
    ) -> Result<f32, Self::Error>;
    fn cached_animatable(
        &mut self,
        entity: &MotionGraphEntityAuthority<K>,
    ) -> Result<Option<Self::Animatable>, Self::Error>;
    fn find_animatable(
        &mut self,
        entity: &MotionGraphEntityAuthority<K>,
    ) -> Result<Option<Self::Animatable>, Self::Error>;
    fn transitioning(
        &mut self,
        entity: &MotionGraphEntityAuthority<K>,
        animatable: &Self::Animatable,
    ) -> Result<bool, Self::Error>;
    fn anim_property(
        &mut self,
        entity: &MotionGraphEntityAuthority<K>,
        animatable: &Self::Animatable,
    ) -> Result<Option<AnimProperty>, Self::Error>;
}

/// `PlayAnimation` services whose target authority is supplied on every call.
#[allow(clippy::missing_errors_doc)]
pub trait MotionGraphPlayAnimationRuntime<K> {
    type Error;

    fn resolve_interface(
        &mut self,
        entity: &MotionGraphEntityAuthority<K>,
        kind: PlayAnimationInterfaceKind,
    ) -> Result<PlayAnimationInterfaceResolution, Self::Error>;
    fn motion_intent(
        &mut self,
        entity: &MotionGraphEntityAuthority<K>,
        name: FastString36,
    ) -> Result<Option<u32>, Self::Error>;
    fn filtered_motion_intent(
        &mut self,
        entity: &MotionGraphEntityAuthority<K>,
        name: FastString36,
    ) -> Result<Option<u32>, Self::Error>;
    fn tree_attribute(
        &mut self,
        entity: &MotionGraphEntityAuthority<K>,
        name: FastString30,
    ) -> Result<Option<TreeAttributeSample>, Self::Error>;
    fn add_settable_attribute(
        &mut self,
        entity: &MotionGraphEntityAuthority<K>,
        name: FastString30,
        value_bits: u32,
        normalize: bool,
        sequence: i32,
    ) -> Result<(), Self::Error>;
    fn selected_pro_skater(
        &mut self,
        entity: &MotionGraphEntityAuthority<K>,
    ) -> Result<FastString30, Self::Error>;
    fn set_construction_attribute(
        &mut self,
        entity: &MotionGraphEntityAuthority<K>,
        name: FastString30,
        value: FastString30,
    ) -> Result<(), Self::Error>;
    fn next_transition_override(
        &mut self,
        entity: &MotionGraphEntityAuthority<K>,
    ) -> Result<Option<RawAnimationTransitionParams>, Self::Error>;
    fn clear_transition_override(
        &mut self,
        entity: &MotionGraphEntityAuthority<K>,
    ) -> Result<(), Self::Error>;
    fn is_switch(&mut self, entity: &MotionGraphEntityAuthority<K>) -> Result<bool, Self::Error>;
    fn is_mirrored(&mut self, entity: &MotionGraphEntityAuthority<K>) -> Result<bool, Self::Error>;
    fn physical_output(
        &mut self,
        entity: &MotionGraphEntityAuthority<K>,
    ) -> Result<PhysicalOutputObservation, Self::Error>;
    fn dispatch(
        &mut self,
        entity: &MotionGraphEntityAuthority<K>,
        request: AnimationDispatch,
    ) -> Result<u8, Self::Error>;
}

/// Living-world remapped-animation services whose target authority is
/// supplied on every call.
#[allow(clippy::missing_errors_doc)]
pub trait MotionGraphPlayRemappedAnimationRuntime<K> {
    type Error;

    fn write_remapped_mirror_override_raw(
        &mut self,
        entity: &MotionGraphEntityAuthority<K>,
        value: u8,
    ) -> Result<(), Self::Error>;
    fn setup_remapped_animation(
        &mut self,
        entity: &MotionGraphEntityAuthority<K>,
        request: RemappedAnimationSetup<'_>,
    ) -> Result<(), Self::Error>;
    fn update_remapped_animation(
        &mut self,
        entity: &MotionGraphEntityAuthority<K>,
        request: RemappedAnimationUpdate<'_>,
    ) -> Result<(), Self::Error>;
}

/// Owns one immutable Entity authority and its concrete callback services.
///
/// There is no runtime or authority replacement API. The bound session creates
/// a short-lived adapter internally for each operation.
///
/// ```compile_fail
/// use skate2_core::motion_graph_entity_authority::ActorGraphAuthority;
/// let owner = ActorGraphAuthority::new(7_u32);
/// let mut facade = owner.motion_graph_facade(String::new());
/// facade.runtime_mut().push_str("retarget");
/// ```
pub struct MotionGraphRuntimeFacade<K, R> {
    entity: MotionGraphEntityAuthority<K>,
    runtime: R,
}

impl<K, R> MotionGraphRuntimeFacade<K, R> {
    fn from_authority(entity: MotionGraphEntityAuthority<K>, runtime: R) -> Self {
        Self { entity, runtime }
    }

    #[must_use]
    pub fn entity_authority(&self) -> &MotionGraphEntityAuthority<K> {
        &self.entity
    }

    pub(crate) fn borrow(&mut self) -> BorrowedMotionGraphRuntimeFacade<'_, K, R> {
        BorrowedMotionGraphRuntimeFacade {
            entity: &self.entity,
            runtime: &mut self.runtime,
        }
    }
}

pub(crate) struct BorrowedMotionGraphRuntimeFacade<'a, K, R> {
    entity: &'a MotionGraphEntityAuthority<K>,
    runtime: &'a mut R,
}

impl<K, R> crate::motion_graph_conditions::HasIntentCollaborator
    for BorrowedMotionGraphRuntimeFacade<'_, K, R>
where
    R: MotionGraphHasIntentRuntime<K>,
{
    type Error = R::Error;
    type MotionGraph = R::MotionGraph;
    type Intents = R::Intents;

    fn cached_motion_graph(&mut self) -> Result<Option<Self::MotionGraph>, Self::Error> {
        self.runtime.cached_motion_graph(self.entity)
    }

    fn find_motion_graph(&mut self) -> Result<Option<Self::MotionGraph>, Self::Error> {
        self.runtime.find_motion_graph(self.entity)
    }

    fn motion_graph_intents(
        &mut self,
        motion_graph: &Self::MotionGraph,
    ) -> Result<Self::Intents, Self::Error> {
        self.runtime.motion_graph_intents(self.entity, motion_graph)
    }

    fn lookup_intent(
        &mut self,
        intents: &Self::Intents,
        intent_name: &[u8],
    ) -> Result<Option<f32>, Self::Error> {
        self.runtime
            .lookup_intent(self.entity, intents, intent_name)
    }
}

impl<K, R> crate::motion_graph_conditions::WillExpireCollaborator
    for BorrowedMotionGraphRuntimeFacade<'_, K, R>
where
    R: MotionGraphWillExpireRuntime<K>,
{
    type Error = R::Error;
    type AnimInfo = R::AnimInfo;
    type Animatable = R::Animatable;

    fn find_anim_info(&mut self) -> Result<Option<Self::AnimInfo>, Self::Error> {
        self.runtime.find_anim_info(self.entity)
    }

    fn resolve_special_tag(
        &mut self,
        anim_info: &Self::AnimInfo,
        tag: &[u8],
    ) -> Result<f32, Self::Error> {
        self.runtime
            .resolve_special_tag(self.entity, anim_info, tag)
    }

    fn cached_animatable(&mut self) -> Result<Option<Self::Animatable>, Self::Error> {
        self.runtime.cached_animatable(self.entity)
    }

    fn find_animatable(&mut self) -> Result<Option<Self::Animatable>, Self::Error> {
        self.runtime.find_animatable(self.entity)
    }

    fn transitioning(&mut self, animatable: &Self::Animatable) -> Result<bool, Self::Error> {
        self.runtime.transitioning(self.entity, animatable)
    }

    fn anim_property(
        &mut self,
        animatable: &Self::Animatable,
    ) -> Result<Option<AnimProperty>, Self::Error> {
        self.runtime.anim_property(self.entity, animatable)
    }
}

impl<K, R> crate::play_animation_lifecycle::PlayAnimationLifecycleBackend
    for BorrowedMotionGraphRuntimeFacade<'_, K, R>
where
    R: MotionGraphPlayAnimationRuntime<K>,
{
    type Error = R::Error;

    fn resolve_interface(
        &mut self,
        kind: PlayAnimationInterfaceKind,
    ) -> Result<PlayAnimationInterfaceResolution, Self::Error> {
        self.runtime.resolve_interface(self.entity, kind)
    }

    fn motion_intent(&mut self, name: FastString36) -> Result<Option<u32>, Self::Error> {
        self.runtime.motion_intent(self.entity, name)
    }

    fn filtered_motion_intent(&mut self, name: FastString36) -> Result<Option<u32>, Self::Error> {
        self.runtime.filtered_motion_intent(self.entity, name)
    }

    fn tree_attribute(
        &mut self,
        name: FastString30,
    ) -> Result<Option<TreeAttributeSample>, Self::Error> {
        self.runtime.tree_attribute(self.entity, name)
    }

    fn add_settable_attribute(
        &mut self,
        name: FastString30,
        value_bits: u32,
        normalize: bool,
        sequence: i32,
    ) -> Result<(), Self::Error> {
        self.runtime
            .add_settable_attribute(self.entity, name, value_bits, normalize, sequence)
    }

    fn selected_pro_skater(&mut self) -> Result<FastString30, Self::Error> {
        self.runtime.selected_pro_skater(self.entity)
    }

    fn set_construction_attribute(
        &mut self,
        name: FastString30,
        value: FastString30,
    ) -> Result<(), Self::Error> {
        self.runtime
            .set_construction_attribute(self.entity, name, value)
    }

    fn next_transition_override(
        &mut self,
    ) -> Result<Option<RawAnimationTransitionParams>, Self::Error> {
        self.runtime.next_transition_override(self.entity)
    }

    fn clear_transition_override(&mut self) -> Result<(), Self::Error> {
        self.runtime.clear_transition_override(self.entity)
    }

    fn is_switch(&mut self) -> Result<bool, Self::Error> {
        self.runtime.is_switch(self.entity)
    }

    fn is_mirrored(&mut self) -> Result<bool, Self::Error> {
        self.runtime.is_mirrored(self.entity)
    }

    fn physical_output(&mut self) -> Result<PhysicalOutputObservation, Self::Error> {
        self.runtime.physical_output(self.entity)
    }

    fn dispatch(&mut self, request: AnimationDispatch) -> Result<u8, Self::Error> {
        self.runtime.dispatch(self.entity, request)
    }
}

impl<K, R> PlayRemappedAnimationCollaborator for BorrowedMotionGraphRuntimeFacade<'_, K, R>
where
    R: MotionGraphPlayRemappedAnimationRuntime<K>,
{
    type Error = R::Error;

    fn write_mirror_override_raw(&mut self, value: u8) -> Result<(), Self::Error> {
        self.runtime
            .write_remapped_mirror_override_raw(self.entity, value)
    }

    fn setup_remapped_animation(
        &mut self,
        request: RemappedAnimationSetup<'_>,
    ) -> Result<(), Self::Error> {
        self.runtime.setup_remapped_animation(self.entity, request)
    }

    fn update_remapped_animation(
        &mut self,
        request: RemappedAnimationUpdate<'_>,
    ) -> Result<(), Self::Error> {
        self.runtime.update_remapped_animation(self.entity, request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_keys_do_not_collapse_independent_entity_issuances() {
        let first = MotionGraphEntityAuthority::issue(7_u32);
        let first_copy = first.shallow_copy();
        let second = MotionGraphEntityAuthority::issue(7_u32);

        assert_eq!(first.key(), &7);
        assert_eq!(first, first_copy);
        assert_ne!(first, second);
    }

    #[test]
    fn controller_tokens_compare_by_issuance() {
        let first = MotionGraphControllerToken::issue();
        let first_copy = first.shallow_copy();
        let second = MotionGraphControllerToken::issue();

        assert_eq!(first, first_copy);
        assert_ne!(first, second);
    }

    #[test]
    fn bound_context_starts_with_exact_constructor_null_slots() {
        let controller = MotionGraphControllerToken::issue();
        let entity = MotionGraphEntityAuthority::issue(9_u32);
        let context: BoundMotionGraphContext<u32> = StateGraphContext::new(
            Some(controller.shallow_copy()),
            Some(entity.shallow_copy()),
            None,
            None,
            None,
            None,
        );

        assert_eq!(context.controller(), Some(&controller));
        assert_eq!(context.entity(), Some(&entity));
        assert!(context.instance().is_none());
        assert!(context.camera_man().is_none());
        assert!(context.behaviour_source().is_none());
        assert!(context.behaviour_sink().is_none());
    }
}
