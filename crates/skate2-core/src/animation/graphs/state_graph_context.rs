//! Semantic storage for the six native `StateGraph::Context` pointer slots.
//!
//! PDB layout evidence fixes the 32-bit native object at 24 bytes: controller
//! `+0x00`, entity `+0x04`, Instance `+0x08`, `CameraMan` `+0x0c`,
//! `BehaviourSource` `+0x10`, and `BehaviourSink` `+0x14`. Generic dynamic
//! Behaviour paths copy all six words and replace only the copy's Instance:
//! `UpdateEnterStates` `0x82cf4fe8..0x82cf5314`, `UpdateBehaviours`
//! `0x82cef870..0x82cef938`, `UpdateExitStates` `0x82cf3310..0x82cf35b4`,
//! and `EndAllBehaviours` `0x82cf3198..0x82cf32e4`. Transition hooks at
//! `0x82cef938..0x82cef98c` receive the original Context without an instance
//! overlay.
//! `Context::SetBehaviourSource` `0x82ceed30..0x82ceed84` writes `+0x10`, and
//! `Context::SetCurrentBehaviourSink` `0x826fc770..0x826fc7c4` writes `+0x14`;
//! these are the only public slot mutators exposed here. Generated `StateUpdate`
//! wrappers can also install Behaviour instances before direct Update calls,
//! but their complete dispatch/lifetime boundary is not reconstructed by this
//! generic-context module.
//!
//! This Rust type is deliberately not `repr(C)`, `Clone`, or `Copy`. Its
//! generic `Option` fields do not claim native size, alignment, pointer bits,
//! ownership, lifetime, or validity. Handles are opaque semantic tokens; this
//! module does not derive a controller self-pointer, resolve an entity, confer
//! Actor/UCOM authority, or define CameraMan/BehaviourSource/BehaviourSink
//! behavior. The dynamic controller's separately supplied Behaviour instance
//! remains the semantic `+0x08` overlay over a shallow Context copy.

use crate::dynamic_state_graph_controller::DynamicStateGraphContext;

pub const NATIVE_STATE_GRAPH_CONTEXT_SIZE: usize = 0x18;
pub const NATIVE_STATE_GRAPH_CONTEXT_SLOT_SIZE: usize = 0x04;
pub const NATIVE_STATE_GRAPH_CONTEXT_SLOT_COUNT: usize = 6;
pub const NATIVE_STATE_GRAPH_CONTEXT_CONTROLLER_OFFSET: usize = 0x00;
pub const NATIVE_STATE_GRAPH_CONTEXT_ENTITY_OFFSET: usize = 0x04;
pub const NATIVE_STATE_GRAPH_CONTEXT_INSTANCE_OFFSET: usize = 0x08;
pub const NATIVE_STATE_GRAPH_CONTEXT_CAMERA_MAN_OFFSET: usize = 0x0c;
pub const NATIVE_STATE_GRAPH_CONTEXT_BEHAVIOUR_SOURCE_OFFSET: usize = 0x10;
pub const NATIVE_STATE_GRAPH_CONTEXT_BEHAVIOUR_SINK_OFFSET: usize = 0x14;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StateGraphContextSlot {
    Controller,
    Entity,
    Instance,
    CameraMan,
    BehaviourSource,
    BehaviourSink,
}

pub const NATIVE_STATE_GRAPH_CONTEXT_LAYOUT: [(StateGraphContextSlot, usize); 6] = [
    (
        StateGraphContextSlot::Controller,
        NATIVE_STATE_GRAPH_CONTEXT_CONTROLLER_OFFSET,
    ),
    (
        StateGraphContextSlot::Entity,
        NATIVE_STATE_GRAPH_CONTEXT_ENTITY_OFFSET,
    ),
    (
        StateGraphContextSlot::Instance,
        NATIVE_STATE_GRAPH_CONTEXT_INSTANCE_OFFSET,
    ),
    (
        StateGraphContextSlot::CameraMan,
        NATIVE_STATE_GRAPH_CONTEXT_CAMERA_MAN_OFFSET,
    ),
    (
        StateGraphContextSlot::BehaviourSource,
        NATIVE_STATE_GRAPH_CONTEXT_BEHAVIOUR_SOURCE_OFFSET,
    ),
    (
        StateGraphContextSlot::BehaviourSink,
        NATIVE_STATE_GRAPH_CONTEXT_BEHAVIOUR_SINK_OFFSET,
    ),
];

/// Explicit shallow-copy contract for one semantic Context handle.
///
/// The result must identify the same logical referent as `self`; it must not
/// deep-copy, retarget, or perform a gameplay operation. This is a host
/// contract rather than proof that the handle is live or authoritative.
pub trait StateGraphContextHandle: Sized {
    #[must_use]
    fn shallow_copy(&self) -> Self;
}

impl<T: Copy> StateGraphContextHandle for T {
    fn shallow_copy(&self) -> Self {
        *self
    }
}

/// Six separately typed, nullable semantic Context slots.
///
/// `None` represents a native null pointer. A present handle remains opaque;
/// the type does not dereference it or infer ownership from it.
/// Shallow copying stays explicit even when every handle is `Copy`:
///
/// ```compile_fail
/// use skate2_core::state_graph_context::StateGraphContext;
/// let context: StateGraphContext<u8, u16, u32, u64, usize, isize> =
///     StateGraphContext::empty();
/// let _copy = context.clone();
/// ```
pub struct StateGraphContext<
    Controller,
    Entity,
    Instance,
    CameraMan,
    BehaviourSource,
    BehaviourSink,
> {
    controller: Option<Controller>,
    entity: Option<Entity>,
    instance: Option<Instance>,
    camera_man: Option<CameraMan>,
    behaviour_source: Option<BehaviourSource>,
    behaviour_sink: Option<BehaviourSink>,
}

impl<Controller, Entity, Instance, CameraMan, BehaviourSource, BehaviourSink>
    StateGraphContext<Controller, Entity, Instance, CameraMan, BehaviourSource, BehaviourSink>
{
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            controller: None,
            entity: None,
            instance: None,
            camera_man: None,
            behaviour_source: None,
            behaviour_sink: None,
        }
    }

    #[must_use]
    pub const fn new(
        controller: Option<Controller>,
        entity: Option<Entity>,
        instance: Option<Instance>,
        camera_man: Option<CameraMan>,
        behaviour_source: Option<BehaviourSource>,
        behaviour_sink: Option<BehaviourSink>,
    ) -> Self {
        Self {
            controller,
            entity,
            instance,
            camera_man,
            behaviour_source,
            behaviour_sink,
        }
    }

    #[must_use]
    pub const fn controller(&self) -> Option<&Controller> {
        self.controller.as_ref()
    }

    #[must_use]
    pub const fn entity(&self) -> Option<&Entity> {
        self.entity.as_ref()
    }

    #[must_use]
    pub const fn instance(&self) -> Option<&Instance> {
        self.instance.as_ref()
    }

    /// Generated `StateUpdate` callbacks write the actual Context's +0x08 slot.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "generated direct Update dispatch is not yet mounted"
        )
    )]
    pub(crate) fn set_generated_instance(&mut self, value: Option<Instance>) {
        self.instance = value;
    }

    #[must_use]
    pub const fn camera_man(&self) -> Option<&CameraMan> {
        self.camera_man.as_ref()
    }

    #[must_use]
    pub const fn behaviour_source(&self) -> Option<&BehaviourSource> {
        self.behaviour_source.as_ref()
    }

    /// Replaces the nullable `BehaviourSource` slot recovered at `+0x10`.
    pub fn set_behaviour_source(&mut self, value: Option<BehaviourSource>) {
        self.behaviour_source = value;
    }

    #[must_use]
    pub const fn behaviour_sink(&self) -> Option<&BehaviourSink> {
        self.behaviour_sink.as_ref()
    }

    /// Replaces the nullable current `BehaviourSink` slot recovered at `+0x14`.
    pub fn set_current_behaviour_sink(&mut self, value: Option<BehaviourSink>) {
        self.behaviour_sink = value;
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)] // Production impls remain below this local test block.
mod tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct ControllerId(u32);
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct EntityId(u32);
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct InstanceId(u32);
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct CameraId(u32);
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct SourceId(u32);
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct SinkId(u32);

    type IdContext =
        StateGraphContext<ControllerId, EntityId, InstanceId, CameraId, SourceId, SinkId>;

    #[test]
    fn native_size_offsets_and_slot_order_are_frozen_without_rust_layout_claim() {
        assert_eq!(NATIVE_STATE_GRAPH_CONTEXT_SIZE, 0x18);
        assert_eq!(NATIVE_STATE_GRAPH_CONTEXT_SLOT_SIZE, 4);
        assert_eq!(NATIVE_STATE_GRAPH_CONTEXT_SLOT_COUNT, 6);
        assert_eq!(
            NATIVE_STATE_GRAPH_CONTEXT_LAYOUT,
            [
                (StateGraphContextSlot::Controller, 0x00),
                (StateGraphContextSlot::Entity, 0x04),
                (StateGraphContextSlot::Instance, 0x08),
                (StateGraphContextSlot::CameraMan, 0x0c),
                (StateGraphContextSlot::BehaviourSource, 0x10),
                (StateGraphContextSlot::BehaviourSink, 0x14),
            ]
        );
        assert_eq!(
            NATIVE_STATE_GRAPH_CONTEXT_BEHAVIOUR_SINK_OFFSET + NATIVE_STATE_GRAPH_CONTEXT_SLOT_SIZE,
            NATIVE_STATE_GRAPH_CONTEXT_SIZE
        );
    }

    #[test]
    fn empty_and_default_preserve_six_nullable_slots() {
        for context in [IdContext::empty(), IdContext::default()] {
            assert!(context.controller().is_none());
            assert!(context.entity().is_none());
            assert!(context.instance().is_none());
            assert!(context.camera_man().is_none());
            assert!(context.behaviour_source().is_none());
            assert!(context.behaviour_sink().is_none());
        }
    }

    #[test]
    fn mixed_slots_and_recovered_setters_keep_distinct_types() {
        let mut context = IdContext::new(
            Some(ControllerId(1)),
            None,
            Some(InstanceId(3)),
            None,
            Some(SourceId(5)),
            Some(SinkId(6)),
        );
        assert_eq!(context.controller(), Some(&ControllerId(1)));
        assert!(context.entity().is_none());
        assert_eq!(context.instance(), Some(&InstanceId(3)));
        assert!(context.camera_man().is_none());
        assert_eq!(context.behaviour_source(), Some(&SourceId(5)));
        assert_eq!(context.behaviour_sink(), Some(&SinkId(6)));

        context.set_behaviour_source(Some(SourceId(50)));
        assert_eq!(context.behaviour_source(), Some(&SourceId(50)));
        context.set_behaviour_source(None);
        assert!(context.behaviour_source().is_none());

        context.set_current_behaviour_sink(Some(SinkId(60)));
        assert_eq!(context.behaviour_sink(), Some(&SinkId(60)));
        context.set_current_behaviour_sink(None);
        assert!(context.behaviour_sink().is_none());
    }

    #[test]
    fn copy_handles_all_six_present_and_mixed_nullable_slots() {
        let context = IdContext::new(
            Some(ControllerId(1)),
            Some(EntityId(2)),
            Some(InstanceId(3)),
            Some(CameraId(4)),
            Some(SourceId(5)),
            Some(SinkId(6)),
        );
        let copy = context.copy_for_behaviour();
        assert_eq!(copy.controller(), Some(&ControllerId(1)));
        assert_eq!(copy.entity(), Some(&EntityId(2)));
        assert_eq!(copy.instance(), Some(&InstanceId(3)));
        assert_eq!(copy.camera_man(), Some(&CameraId(4)));
        assert_eq!(copy.behaviour_source(), Some(&SourceId(5)));
        assert_eq!(copy.behaviour_sink(), Some(&SinkId(6)));

        let mixed = IdContext::new(
            None,
            Some(EntityId(20)),
            None,
            Some(CameraId(40)),
            None,
            Some(SinkId(60)),
        )
        .copy_for_behaviour();
        assert!(mixed.controller().is_none());
        assert_eq!(mixed.entity(), Some(&EntityId(20)));
        assert!(mixed.instance().is_none());
        assert_eq!(mixed.camera_man(), Some(&CameraId(40)));
        assert!(mixed.behaviour_source().is_none());
        assert_eq!(mixed.behaviour_sink(), Some(&SinkId(60)));
    }

    struct SharedValue(Rc<Cell<u32>>);

    impl StateGraphContextHandle for SharedValue {
        fn shallow_copy(&self) -> Self {
            Self(Rc::clone(&self.0))
        }
    }

    #[test]
    fn generated_enter_overlay_preserves_base_but_update_persists_instance() {
        use crate::motion_graph_entity_authority::{
            BoundMotionGraphContext, MotionGraphInstanceToken,
        };
        let first = MotionGraphInstanceToken::issue();
        let second = MotionGraphInstanceToken::issue();
        let mut context = BoundMotionGraphContext::<u32>::empty();
        context.set_generated_instance(Some(first.shallow_copy()));
        let mut enter = context.copy_for_behaviour();
        enter.set_generated_instance(Some(second.shallow_copy()));
        assert!(context.instance().unwrap().is_same_issuance(&first));
        assert!(enter.instance().unwrap().is_same_issuance(&second));
        context.set_generated_instance(Some(second.shallow_copy()));
        let next_enter = context.copy_for_behaviour();
        assert!(next_enter.instance().unwrap().is_same_issuance(&second));
        assert!(!first.is_same_issuance(&second));
    }

    #[test]
    fn direct_copy_slot_replacement_is_isolated_but_shared_pointee_mutation_is_visible() {
        type SharedContext =
            StateGraphContext<ControllerId, SharedValue, InstanceId, CameraId, SourceId, SinkId>;

        let shared = Rc::new(Cell::new(7));
        let mut context = SharedContext::new(
            Some(ControllerId(1)),
            Some(SharedValue(Rc::clone(&shared))),
            Some(InstanceId(3)),
            Some(CameraId(4)),
            Some(SourceId(5)),
            Some(SinkId(6)),
        );
        let mut copy = context.copy_for_behaviour();
        assert!(Rc::ptr_eq(
            &context.entity().unwrap().0,
            &copy.entity().unwrap().0
        ));

        copy.set_behaviour_source(Some(SourceId(55)));
        copy.set_current_behaviour_sink(Some(SinkId(66)));
        assert_eq!(context.behaviour_source(), Some(&SourceId(5)));
        assert_eq!(copy.behaviour_source(), Some(&SourceId(55)));
        assert_eq!(context.behaviour_sink(), Some(&SinkId(6)));
        assert_eq!(copy.behaviour_sink(), Some(&SinkId(66)));

        copy.entity().unwrap().0.set(70);
        assert_eq!(context.entity().unwrap().0.get(), 70);
        assert_eq!(shared.get(), 70);

        // The base Instance slot is copied unchanged. The dynamic controller's
        // separately owned callback instance shadows this value; it is not
        // installed or mutated by StateGraphContext itself.
        assert_eq!(context.instance(), Some(&InstanceId(3)));
        assert_eq!(copy.instance(), Some(&InstanceId(3)));

        context.set_behaviour_source(Some(SourceId(500)));
        context.set_current_behaviour_sink(Some(SinkId(600)));
        let later_copy = context.copy_for_behaviour();
        assert_eq!(later_copy.behaviour_source(), Some(&SourceId(500)));
        assert_eq!(later_copy.behaviour_sink(), Some(&SinkId(600)));
    }
}

impl<Controller, Entity, Instance, CameraMan, BehaviourSource, BehaviourSink> Default
    for StateGraphContext<Controller, Entity, Instance, CameraMan, BehaviourSource, BehaviourSink>
{
    fn default() -> Self {
        Self::empty()
    }
}

fn copy_nullable<Handle: StateGraphContextHandle>(value: Option<&Handle>) -> Option<Handle> {
    value.map(StateGraphContextHandle::shallow_copy)
}

impl<Controller, Entity, Instance, CameraMan, BehaviourSource, BehaviourSink>
    DynamicStateGraphContext
    for StateGraphContext<Controller, Entity, Instance, CameraMan, BehaviourSource, BehaviourSink>
where
    Controller: StateGraphContextHandle,
    Entity: StateGraphContextHandle,
    Instance: StateGraphContextHandle,
    CameraMan: StateGraphContextHandle,
    BehaviourSource: StateGraphContextHandle,
    BehaviourSink: StateGraphContextHandle,
{
    fn copy_for_behaviour(&self) -> Self {
        Self {
            controller: copy_nullable(self.controller.as_ref()),
            entity: copy_nullable(self.entity.as_ref()),
            instance: copy_nullable(self.instance.as_ref()),
            camera_man: copy_nullable(self.camera_man.as_ref()),
            behaviour_source: copy_nullable(self.behaviour_source.as_ref()),
            behaviour_sink: copy_nullable(self.behaviour_sink.as_ref()),
        }
    }
}
