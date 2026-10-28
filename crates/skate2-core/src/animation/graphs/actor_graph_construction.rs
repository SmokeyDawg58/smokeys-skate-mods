//! Concrete unloaded graph facades constructed after the same Actor listener.
//! Source constructors `0x82cea340`, `0x82cea060`, `0x825fdda8`; no load or tick.
//!
//! Host storage preserves constructor state, identity and registration order.
//! It does not implement native Sim/`StateGraph` allocators.
//! Host `Vec` reservations can fail where native inline stores
//! cannot; those checked host failures retain the initialized prefix.
#![expect(
    dead_code,
    reason = "graph facade fields await subsequent Actor startup stages"
)]
use crate::{
    motion_graph_entity_authority::{
        ActorGraphAuthority, BoundMotionGraphContext, MotionGraphControllerToken,
        MotionGraphEntityAuthority,
    },
    play_animation::{EMPTY_FAST_STRING_36, FastString30, FastString36},
    state_graph_context::{StateGraphContext, StateGraphContextHandle},
};
use std::{fmt, sync::Arc};
#[path = "actor_graph_motion_attributes.rs"]
mod motion_attributes;
#[cfg(test)]
#[path = "actor_graph_construction_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ActorGraphInterface {
    Action,
    Motion,
    SkaterMotion,
}
impl ActorGraphInterface {
    pub(crate) const fn source_name(self) -> u32 {
        match self {
            Self::Action => 0x825e_be88,
            Self::Motion => 0x825e_c090,
            Self::SkaterMotion => 0x825e_c0a0,
        }
    }
}
/// Required bridge to the Actor's existing interface registry, never a second
/// registry. Implementations must reject a foreign Entity issuance before Add.
pub(crate) trait ActorGraphRegistry<K> {
    type Error;
    fn register_graph_interface(
        &mut self,
        entity: &MotionGraphEntityAuthority<K>,
        kind: ActorGraphInterface,
        component: &Arc<()>,
    ) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub(crate) enum ActorGraphConstructionError<E> {
    AlreadyAttempted,
    ActionRequired,
    Allocation,
    Registry(E),
}
impl<E: fmt::Debug> fmt::Display for ActorGraphConstructionError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Registry(error) => write!(f, "Actor graph registry: {error:?}"),
            other => write!(f, "Actor graph construction: {other:?}"),
        }
    }
}

/// Explicit allocation outcome at the selected source allocation boundary.
/// `Host` constructs retained typed storage using the host allocator; `Null`
/// reproduces the native successful null branch. Host allocator/category and
/// out-of-memory behavior are not asserted to match Xenon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ActorGraphAllocation {
    Host,
    Null,
}

/// Constructor wrapper at `0x82cea3c0` / `0x82cea104`.
pub(crate) enum RetainedHierarchicalController<K> {
    Action(Arc<std::sync::Mutex<crate::generated_action_load::GeneratedActionController<K>>>),
    Motion(
        Arc<
            std::sync::Mutex<
                crate::generated_motion_load::controller::GeneratedMotionController<K>,
            >,
        >,
    ),
}
pub(crate) struct ActorHierarchicalWrapper<K> {
    context: BoundMotionGraphContext<K>,
    self_controller: MotionGraphControllerToken,
    root_name_hash: u32,
    // Native AbstractHierarchicalController remains null until Load.
    abstract_controller: Option<RetainedHierarchicalController<K>>,
}

#[path = "actor_graph_load.rs"]
pub(crate) mod load;
#[path = "actor_graph_motion_load.rs"]
pub(crate) mod motion_load;
impl<K> ActorHierarchicalWrapper<K> {
    fn allocate(
        entity: &MotionGraphEntityAuthority<K>,
        allocation: ActorGraphAllocation,
    ) -> Option<Self> {
        match allocation {
            ActorGraphAllocation::Host => Some(Self::new(entity)),
            ActorGraphAllocation::Null => None,
        }
    }
    fn new(entity: &MotionGraphEntityAuthority<K>) -> Self {
        let self_controller = MotionGraphControllerToken::issue();
        Self {
            context: StateGraphContext::new(
                Some(self_controller.shallow_copy()),
                Some(entity.shallow_copy()),
                None,
                None,
                None,
                None,
            ),
            self_controller,
            root_name_hash: 0,
            abstract_controller: None,
        }
    }
    pub(crate) fn context(&self) -> &BoundMotionGraphContext<K> {
        &self.context
    }
    /// Direct generated Update mutates only the original Context instance slot.
    pub(crate) fn set_runtime_instance(
        &mut self,
        instance: Option<crate::motion_graph_entity_authority::MotionGraphInstanceToken>,
    ) {
        self.context.set_generated_instance(instance);
    }
    pub(crate) fn controller_identity(&self) -> &MotionGraphControllerToken {
        &self.self_controller
    }
    pub(crate) const fn root_name_hash(&self) -> u32 {
        self.root_name_hash
    }
    pub(crate) fn is_unloaded(&self) -> bool {
        self.abstract_controller.is_none()
    }
}

#[path = "actor_graph_intents.rs"]
mod intents;
pub(crate) use intents::{ActorGraphIntentError, ActorGraphIntents};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Only `known_mask` bits are established; zero in other bits is not evidence.
pub(crate) struct ActorGraphKnownBits {
    pub(crate) known_mask: u32,
    pub(crate) bits: u32,
}
impl ActorGraphKnownBits {
    fn clear(mask: u32) -> Self {
        Self {
            known_mask: mask,
            bits: 0,
        }
    }
    fn assign(&mut self, mask: u32, bits: u32) {
        self.bits = (self.bits & !mask) | (bits & mask);
        self.known_mask |= mask;
    }
}
pub(crate) struct ActorGraphTransition {
    pub(crate) transition_type: u32,
    pub(crate) blend_length: f32,
    pub(crate) placement: u32,
    pub(crate) flags: ActorGraphKnownBits,
}
impl ActorGraphTransition {
    pub(crate) fn new() -> Self {
        Self {
            transition_type: 0,
            blend_length: f32::from_bits(0x3e4c_cccd),
            placement: 0,
            flags: ActorGraphKnownBits::clear(0xc000_0000),
        }
    }
    /// `ClearNextAnimTransOveride`, `0x825fdf50` (base-this).
    pub(crate) fn clear_override(&mut self) {
        self.placement = 0;
        self.blend_length = f32::from_bits(0x3e4c_cccd);
        self.transition_type = 0;
        // Raw df98 copies a temporary stack word, not previous graph low bits.
        self.flags = ActorGraphKnownBits::clear(0xc000_0000);
    }
}
pub(crate) struct ActorMgAttribute {
    pub(crate) name: FastString30,
    pub(crate) value: f32,
}
pub(crate) struct ActorSkaterGraphState {
    attributes: Vec<ActorMgAttribute>,
    pub(crate) last_good_landing_velocity: f32,
    pub(crate) grab_busy: [i32; 2],
    pub(crate) push: [f32; 8],
    pub(crate) push_bits: ActorGraphKnownBits,
    pub(crate) slide_time: [f32; 2],
    pub(crate) slide_bits: [ActorGraphKnownBits; 2],
    pub(crate) combined_slide_bits: ActorGraphKnownBits,
    pub(crate) time_since_kickturn: f32,
    pub(crate) time_since_teleport: f32,
    pub(crate) manual_out_timer: f32,
    pub(crate) flags: ActorGraphKnownBits,
    pub(crate) trick_name: FastString36,
    pub(crate) trick_gesture: FastString36,
    pub(crate) trick_tweak: [f32; 2],
    pub(crate) trick_bits: u32,
    pub(crate) gesture: u32,
    pub(crate) is_gesturing: bool,
    pub(crate) wipeout: [f32; 2],
    pub(crate) random_index: u32,
    pub(crate) random_seed: u32,
    pub(crate) random_values: [u32; 6],
}
impl ActorSkaterGraphState {
    fn try_new<E>() -> Result<Self, ActorGraphConstructionError<E>> {
        let mut attributes = Vec::new();
        attributes
            .try_reserve_exact(20)
            .map_err(|_| ActorGraphConstructionError::Allocation)?;
        Ok(Self {
            attributes,
            last_good_landing_velocity: 0.0,
            grab_busy: [0; 2],
            push: [0.0; 8],
            push_bits: ActorGraphKnownBits::clear(0x8000_0000),
            slide_time: [0.0; 2],
            slide_bits: [ActorGraphKnownBits::clear(0xe000_0000); 2],
            combined_slide_bits: ActorGraphKnownBits::clear(0xc000_0000),
            time_since_kickturn: 0.0,
            time_since_teleport: 0.0,
            manual_out_timer: 0.0,
            flags: ActorGraphKnownBits {
                known_mask: 0xff00_0000,
                bits: 0x0300_0000,
            },
            trick_name: EMPTY_FAST_STRING_36,
            trick_gesture: EMPTY_FAST_STRING_36,
            trick_tweak: [0.0; 2],
            trick_bits: 0,
            gesture: 0x23,
            is_gesturing: false,
            wipeout: [0.0; 2],
            random_index: 0,
            random_seed: 0,
            random_values: crate::native_skater_random::NativeSkaterRandom::reset().values,
        })
    }
    /// Scalar/vector portion of `SkaterMotionGraphReset`, `0x825f6110`.
    fn reset(&mut self) {
        self.attributes.clear();
        self.last_good_landing_velocity = 0.0;
        self.grab_busy = [0; 2];
        self.push[0] = 0.0;
        self.push_bits.assign(0x8000_0000, 0);
        self.push[1..].fill(0.0);
        for wheel in 0..2 {
            self.slide_time[wheel] = 0.0;
            self.slide_bits[wheel].assign(0xe000_0000, 0);
        }
        self.combined_slide_bits.assign(0xc000_0000, 0);
        self.time_since_kickturn = 0.0;
        self.flags.assign(0xfe00_0000, 0x0200_0000);
    }
    pub(crate) fn attributes(&self) -> &[ActorMgAttribute] {
        &self.attributes
    }
    /// `ClearMgAttributes825fe1d0` copies vector begin to end, no payload Reset.
    pub(crate) fn clear_attributes(&mut self) {
        self.attributes.clear();
    }
    /// `SkaterMotionGraph::GetPhysUpdateData`825f7e50. Copy each payload
    /// before resetting its source; intents and attributes are separate stages.
    pub(crate) fn transfer_physics_payloads(
        &mut self,
        destination: &mut crate::actor_construction::anim_out_phys_in::AnimOutPhysInOwner,
    ) {
        destination.trick = crate::physical_player_phys_out::trick::AnimOutTrickData {
            name: self.trick_name,
            gesture: self.trick_gesture,
            bits: self.trick_bits,
            tweak_x: self.trick_tweak[0],
            tweak_y: self.trick_tweak[1],
        };
        self.trick_name = EMPTY_FAST_STRING_36;
        self.trick_gesture = EMPTY_FAST_STRING_36;
        self.trick_bits = 0;
        self.trick_tweak = [0.0; 2];
        destination.gesture = self.gesture;
        destination.gesturing = self.is_gesturing;
        self.gesture = 0x23;
        self.is_gesturing = false;
        destination.wipeout_gesture = self.wipeout;
        self.wipeout = [0.0; 2];
    }
    #[expect(
        clippy::unused_self,
        reason = "fixed capacity is queried alongside the retained store length"
    )]
    pub(crate) const fn attribute_maximum(&self) -> usize {
        20
    }
}

pub(crate) struct ActorActionGraph<K> {
    occurrence: Arc<()>,
    pub(crate) intents: Option<ActorGraphIntents>,
    pub(crate) controller: Option<ActorHierarchicalWrapper<K>>,
    complete: bool,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ActorActionGraphResetError {
    Incomplete,
    MissingIntents,
}
impl<K> ActorActionGraph<K> {
    pub(crate) const fn is_complete(&self) -> bool {
        self.complete
    }
    pub(crate) fn occurrence(&self) -> &Arc<()> {
        &self.occurrence
    }
    /// `ActionGraph::Reset` 0x82cea2b0 frees the same fixed hash's nodes and
    /// zeros its count. The existing typed intents owner retains that hash.
    pub(crate) fn try_reset_live(&mut self) -> Result<(), ActorActionGraphResetError> {
        if !self.complete {
            return Err(ActorActionGraphResetError::Incomplete);
        }
        self.intents
            .as_mut()
            .ok_or(ActorActionGraphResetError::MissingIntents)?
            .clear();
        Ok(())
    }
}
pub(crate) struct ActorSkaterMotionGraph<K> {
    occurrence: Arc<()>,
    pub(crate) intents: Option<ActorGraphIntents>,
    pub(crate) controller: Option<ActorHierarchicalWrapper<K>>,
    pub(crate) transition: Option<ActorGraphTransition>,
    pub(crate) filtered_intents: Option<ActorGraphIntents>,
    pub(crate) state: Option<ActorSkaterGraphState>,
    complete: bool,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ActorSkaterMotionGraphResetError {
    Incomplete,
    Missing(&'static str),
}
impl<K> ActorSkaterMotionGraph<K> {
    pub(crate) const fn is_complete(&self) -> bool {
        self.complete
    }
    /// Shared allocation identity; `IMotionGraph` and `ISkaterMotionGraph` remain
    /// distinct registry kinds (the latter is native base + 0xa78).
    pub(crate) fn occurrence(&self) -> &Arc<()> {
        &self.occurrence
    }
    /// `SkaterMotionGraphReset` 0x825f6110. The derived hash clears first,
    /// then the attribute/scalar block, base intent hash, and transition
    /// override. These are the same retained objects built by the constructor.
    pub(crate) fn try_reset_live(&mut self) -> Result<(), ActorSkaterMotionGraphResetError> {
        use ActorSkaterMotionGraphResetError as E;
        if !self.complete {
            return Err(E::Incomplete);
        }
        self.filtered_intents
            .as_mut()
            .ok_or(E::Missing("derived filtered intents"))?
            .clear(); // 0x825f612c..6138
        self.state
            .as_mut()
            .ok_or(E::Missing("SkaterMotionGraph state"))?
            .reset(); // 0x825f6148..61dc, including ClearMgAttributes
        self.intents
            .as_mut()
            .ok_or(E::Missing("base MotionGraph intents"))?
            .clear(); // 0x825f61e8..61ec
        self.transition
            .as_mut()
            .ok_or(E::Missing("base MotionGraph transition"))?
            .clear_override(); // 0x825f6200, vtable +0x20
        Ok(())
    }
}
pub(crate) struct ActorGraphComponents<K> {
    entity: MotionGraphEntityAuthority<K>,
    action: Option<ActorActionGraph<K>>,
    motion: Option<ActorSkaterMotionGraph<K>>,
    action_finished: bool,
    motion_finished: bool,
}
impl<K> ActorGraphComponents<K> {
    pub(crate) fn new(authority: &ActorGraphAuthority<K>) -> Self {
        Self {
            entity: authority.entity_handle(),
            action: None,
            motion: None,
            action_finished: false,
            motion_finished: false,
        }
    }
    /// A completed null allocation also finishes the source caller stage.
    pub(crate) const fn action_finished(&self) -> bool {
        self.action_finished
    }
    pub(crate) const fn motion_finished(&self) -> bool {
        self.motion_finished
    }
    pub(crate) fn action_mut(&mut self) -> Option<&mut ActorActionGraph<K>> {
        self.action.as_mut()
    }
    pub(crate) fn action(&self) -> Option<&ActorActionGraph<K>> {
        self.action.as_ref()
    }
    pub(crate) fn motion(&self) -> Option<&ActorSkaterMotionGraph<K>> {
        self.motion.as_ref()
    }
    pub(crate) fn motion_mut(&mut self) -> Option<&mut ActorSkaterMotionGraph<K>> {
        self.motion.as_mut()
    }
    pub(crate) fn try_construct_action<R: ActorGraphRegistry<K>>(
        &mut self,
        registry: &mut R,
    ) -> Result<(), ActorGraphConstructionError<R::Error>> {
        self.try_construct_action_with_allocations(
            registry,
            ActorGraphAllocation::Host,
            ActorGraphAllocation::Host,
        )
    }
    /// Actor allocation `0x825fec6c`, constructor `0x82cea340` and wrapper allocation.
    pub(crate) fn try_construct_action_with_allocations<R: ActorGraphRegistry<K>>(
        &mut self,
        registry: &mut R,
        outer: ActorGraphAllocation,
        wrapper: ActorGraphAllocation,
    ) -> Result<(), ActorGraphConstructionError<R::Error>> {
        if self.action.is_some() || self.action_finished {
            return Err(ActorGraphConstructionError::AlreadyAttempted);
        }
        if outer == ActorGraphAllocation::Null {
            self.action_finished = true;
            return Ok(());
        }
        self.action = Some(ActorActionGraph {
            occurrence: Arc::new(()),
            intents: None,
            controller: None,
            complete: false,
        });
        let action = self.action.as_mut().expect("retained allocation");
        registry
            .register_graph_interface(
                &self.entity,
                ActorGraphInterface::Action,
                &action.occurrence,
            )
            .map_err(ActorGraphConstructionError::Registry)?;
        action.intents = Some(ActorGraphIntents::try_new()?);
        action.controller = ActorHierarchicalWrapper::allocate(&self.entity, wrapper);
        action.complete = true;
        self.action_finished = true;
        Ok(())
    }
    pub(crate) fn try_construct_skater_motion<R: ActorGraphRegistry<K>>(
        &mut self,
        registry: &mut R,
    ) -> Result<(), ActorGraphConstructionError<R::Error>> {
        self.try_construct_skater_motion_with_allocations(
            registry,
            ActorGraphAllocation::Host,
            ActorGraphAllocation::Host,
        )
    }
    /// Actor allocation `0x825feca4`, derived constructor `0x825fdda8`.
    pub(crate) fn try_construct_skater_motion_with_allocations<R: ActorGraphRegistry<K>>(
        &mut self,
        registry: &mut R,
        outer: ActorGraphAllocation,
        wrapper: ActorGraphAllocation,
    ) -> Result<(), ActorGraphConstructionError<R::Error>> {
        if !self.action_finished {
            return Err(ActorGraphConstructionError::ActionRequired);
        }
        if self.motion.is_some() || self.motion_finished {
            return Err(ActorGraphConstructionError::AlreadyAttempted);
        }
        if outer == ActorGraphAllocation::Null {
            self.motion_finished = true;
            return Ok(());
        }
        self.motion = Some(ActorSkaterMotionGraph {
            occurrence: Arc::new(()),
            intents: None,
            controller: None,
            transition: None,
            filtered_intents: None,
            state: None,
            complete: false,
        });
        let motion = self.motion.as_mut().expect("retained allocation");
        registry
            .register_graph_interface(
                &self.entity,
                ActorGraphInterface::Motion,
                &motion.occurrence,
            )
            .map_err(ActorGraphConstructionError::Registry)?;
        motion.intents = Some(ActorGraphIntents::try_new()?);
        motion.transition = Some(ActorGraphTransition::new());
        motion.controller = ActorHierarchicalWrapper::allocate(&self.entity, wrapper);
        registry
            .register_graph_interface(
                &self.entity,
                ActorGraphInterface::SkaterMotion,
                &motion.occurrence,
            )
            .map_err(ActorGraphConstructionError::Registry)?;
        motion.filtered_intents = Some(ActorGraphIntents::try_new()?);
        motion.state = Some(ActorSkaterGraphState::try_new()?);
        // Constructor's actual Reset: filtered clear -> attributes/scalars ->
        // base intents clear -> transition reset. No virtual success stubs.
        motion.filtered_intents.as_mut().expect("filtered").clear();
        motion.state.as_mut().expect("state").reset();
        motion.intents.as_mut().expect("base intents").clear();
        motion
            .transition
            .as_mut()
            .expect("transition")
            .clear_override();
        motion.complete = true;
        self.motion_finished = true;
        Ok(())
    }
}
