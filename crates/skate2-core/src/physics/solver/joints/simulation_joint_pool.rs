//! Safe semantic model of the preallocated `rw::physics::Simulation` Joint pool.
//!
//! `Simulation::AddJoint` (`0x82ae4358..0x82ae43ec`) owns two circular
//! sentinels, acquires the Free head, appends to the Active tail, and stores the
//! API parent in native `m_bodyB` and child in `m_bodyA`. `RemoveJoint`
//! (`0x82ae3308..0x82ae335c`) returns an Active slot to the Free tail without
//! clearing any payload. This module preserves those source-level list, count,
//! mapping, and retention rules with indices instead of native pointers.
//!
//! Native frame/limit/body fields are nonowning pointers. The host retains
//! frame and limit payloads in [`Arc`] values, including while a removed slot
//! remains Free, as an explicit lifetime-safety extension. Native initial tag
//! and payload bits are allocator-dependent; safe host slots begin with absent
//! payloads, tag zero, and spy zero. Add never writes tag, removal clears
//! nothing, and reuse overwrites everything except tag.
//! Payload values are constrained to [`Copy`], which excludes user `Drop`
//! implementations from running when a reused slot releases its stale Arc.
//! Arc refcount/deallocation behavior remains host ownership policy.
//!
//! Joint handles carry process-unique authority and a non-reused occurrence.
//! Those checks are host-only ABA protection. There is no implicit removal on
//! Drop. Normal Assembly teardown removes bodies before joints, so joint
//! removal deliberately neither dereferences nor validates stored body handles.

use crate::simulation_rigid_body_pool::{
    SimulationRigidBodyHandle, SimulationRigidBodyIdentity, SimulationRigidBodyObservation,
};
use std::{
    error::Error,
    fmt,
    num::NonZeroU64,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

static NEXT_JOINT_POOL_AUTHORITY: AtomicU64 = AtomicU64::new(1);

/// Shared source `JointLimits` storage; native `SetNormalJointLimits` mutates the
/// definition owner retained by live Joints (0x82b3bb90). First admission binds
/// storage permanently to one pool, preventing cross-pool writes during Build.
/// No writable lock or arbitrary mutation callback is exposed.
#[derive(Clone, Debug)]
pub(crate) struct SimulationMutableJointLimits<L = SimulationJointLimits>(
    Arc<Mutex<MutableJointLimitsState<L>>>,
);

#[derive(Debug)]
struct MutableJointLimitsState<L> {
    value: L,
    pool: Option<SimulationJointPoolAuthority>,
}

impl<L: Copy> SimulationMutableJointLimits<L> {
    pub(crate) fn new(value: L) -> Self {
        Self(Arc::new(Mutex::new(MutableJointLimitsState {
            value,
            pool: None,
        })))
    }

    pub(crate) fn try_read(&self) -> Result<L, SimulationJointPoolError> {
        Ok(self
            .0
            .lock()
            .map_err(|_| SimulationJointPoolError::LimitsLockPoisoned)?
            .value)
    }
}

#[derive(Debug)]
enum RetainedJointLimits<L> {
    Immutable(Arc<L>),
    Mutable(SimulationMutableJointLimits<L>),
}

impl<L: Copy> RetainedJointLimits<L> {
    fn try_read(&self) -> Result<L, SimulationJointPoolError> {
        match self {
            Self::Immutable(value) => Ok(**value),
            Self::Mutable(value) => value.try_read(),
        }
    }
}

struct JointAddPlan {
    id: SimulationJointId,
    occurrence: NonZeroU64,
    next_occurrence: Option<NonZeroU64>,
}

#[cfg(test)]
#[path = "../simulation_mutable_joint_limits_tests.rs"]
pub(crate) mod mutable_limits_tests;

/// Exact five-vector payload of native `rw::physics::JointFrames`.
///
/// PDB size `0x50`, alignment `0x10`. The names retain the PDB's A/B/L roles;
/// this value type does not infer stronger ownership or limit semantics.
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimulationJointFrames {
    pub quaternion_a: [f32; 4],
    pub position_a: [f32; 4],
    pub quaternion_b: [f32; 4],
    pub position_b: [f32; 4],
    /// Native PDB `mQuatL`.
    pub quaternion_l: [f32; 4],
}

/// Exact field layout of native `rw::physics::JointLimits`.
///
/// PDB size `0x40`, alignment `0x10`. `p_prism`, `v_prism`, and the compact
/// scalar suffixes deliberately preserve the source names without recasting
/// them as generic min/max values. Swing/Twist discriminants remain raw so
/// unknown values follow Build's proven unsigned dispatch rather than being
/// rejected by a host enum.
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimulationJointLimits {
    pub p_prism: [f32; 4],
    pub v_prism: [f32; 4],
    pub v_twist: f32,
    pub v_swing: f32,
    pub swing_a: f32,
    pub twist_a: f32,
    pub swing_c: f32,
    pub twist_c: f32,
    /// Raw `rw::physics::SwingType`; known values are 0 through 4.
    pub raw_swing_type: u32,
    /// Raw `rw::physics::TwistType`; known values are 0 through 2.
    pub raw_twist_type: u32,
}

/// Opaque identity of one joint-pool instance.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SimulationJointPoolAuthority(u64);

/// Stable host ordinal of one preallocated Joint slot.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SimulationJointId(u32);

impl SimulationJointId {
    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }
}

/// Copy-only capability naming one allocation occurrence of one Joint slot.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SimulationJointHandle {
    authority: SimulationJointPoolAuthority,
    id: SimulationJointId,
    occurrence: NonZeroU64,
}

impl SimulationJointHandle {
    #[must_use]
    pub const fn id(self) -> SimulationJointId {
        self.id
    }

    /// Removes mutation authority while retaining the allocation occurrence
    /// required to reject stale prepared solver records.
    #[must_use]
    pub(crate) const fn identity(self) -> SimulationJointIdentity {
        SimulationJointIdentity {
            authority: self.authority,
            id: self.id,
            occurrence: self.occurrence,
        }
    }
}

/// Occurrence-qualified read identity for one active Joint slot.
///
/// This private value is never accepted by removal and grants no pool access.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct SimulationJointIdentity {
    authority: SimulationJointPoolAuthority,
    id: SimulationJointId,
    occurrence: NonZeroU64,
}

impl SimulationJointIdentity {
    #[must_use]
    pub(crate) const fn id(self) -> SimulationJointId {
        self.id
    }
}

/// Occurrence-qualified endpoints expected by one prepared Joint record.
///
/// This copied value grants no removal, payload, or RF-row capability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SimulationJointBuildBinding {
    joint: SimulationJointIdentity,
    body_a: SimulationRigidBodyObservation,
    body_b: SimulationRigidBodyObservation,
}

impl SimulationJointBuildBinding {
    #[must_use]
    pub(crate) const fn new(
        joint: SimulationJointIdentity,
        body_a: SimulationRigidBodyObservation,
        body_b: SimulationRigidBodyObservation,
    ) -> Self {
        Self {
            joint,
            body_a,
            body_b,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationJointBuildEndpoint {
    Parent,
    Child,
}

/// Attribution for one nonallocating prepared-Joint binding validation pass.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SimulationJointBuildBindingValidationError {
    Binding {
        ordinal: usize,
        joint: SimulationJointIdentity,
        source: SimulationJointPoolError,
    },
    EndpointMismatch {
        ordinal: usize,
        joint: SimulationJointIdentity,
        endpoint: SimulationJointBuildEndpoint,
    },
}

/// One of the two native Joint rings.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SimulationJointList {
    Free,
    Active,
}

impl SimulationJointList {
    const ALL: [Self; 2] = [Self::Free, Self::Active];

    const fn index(self) -> usize {
        match self {
            Self::Free => 0,
            Self::Active => 1,
        }
    }
}

/// A link target exposed through read-only snapshots.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationJointNode {
    Anchor(SimulationJointList),
    Joint(SimulationJointId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Links {
    right: SimulationJointNode,
    left: SimulationJointNode,
}

/// Native-width Joint-list counts.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SimulationJointCounts {
    pub free: u32,
    pub active: u32,
}

impl SimulationJointCounts {
    fn get(self, list: SimulationJointList) -> u32 {
        match list {
            SimulationJointList::Free => self.free,
            SimulationJointList::Active => self.active,
        }
    }
}

#[derive(Debug)]
struct JointSlot<F, L> {
    id: SimulationJointId,
    occurrence: Option<NonZeroU64>,
    frames: Option<Arc<F>>,
    limits: Option<RetainedJointLimits<L>>,
    links: Links,
    body_a: Option<SimulationRigidBodyHandle>,
    body_b: Option<SimulationRigidBodyHandle>,
    tag: u32,
    spy: u32,
    membership: SimulationJointList,
}

impl<F, L> JointSlot<F, L> {
    fn initialized(id: SimulationJointId, links: Links) -> Self {
        Self {
            id,
            occurrence: None,
            frames: None,
            limits: None,
            links,
            body_a: None,
            body_b: None,
            tag: 0,
            spy: 0,
            membership: SimulationJointList::Free,
        }
    }
}

/// Read-only semantic snapshot of one Joint slot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SimulationJointSlotSnapshot {
    pub id: SimulationJointId,
    pub occurrence: Option<u64>,
    pub frames_present: bool,
    pub limits_present: bool,
    pub right_link: SimulationJointNode,
    pub left_link: SimulationJointNode,
    /// Inert pool-qualified identity for native `m_bodyA`: the API child body.
    pub body_a: Option<SimulationRigidBodyIdentity>,
    /// Inert pool-qualified identity for native `m_bodyB`: the API parent body.
    pub body_b: Option<SimulationRigidBodyIdentity>,
    pub tag: u32,
    pub spy: u32,
    pub membership: SimulationJointList,
}

/// Complete read-only list and slot snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimulationJointPoolSnapshot {
    pub authority: SimulationJointPoolAuthority,
    pub counts: SimulationJointCounts,
    pub next_occurrence: Option<u64>,
    pub free_order: Vec<SimulationJointId>,
    pub active_order: Vec<SimulationJointId>,
    pub slots: Vec<SimulationJointSlotSnapshot>,
}

/// Borrowed retained payload and inert body-identity mapping for one active Joint.
/// This view never reconstructs a body removal capability.
#[derive(Clone, Copy, Debug)]
pub struct SimulationJointBinding<'a, F, L> {
    pub frames: &'a Arc<F>,
    pub limits: &'a Arc<L>,
    /// Inert pool-qualified identity for native `m_bodyA`: the API child body.
    pub body_a: SimulationRigidBodyIdentity,
    /// Inert pool-qualified identity for native `m_bodyB`: the API parent body.
    pub body_b: SimulationRigidBodyIdentity,
    pub tag: u32,
    pub spy: u32,
}

/// Borrowed view of one exact-payload Active Joint occurrence.
///
/// Accessors return only copied identities, observations, and values. The
/// retained slot, native-shaped links, Arcs, and removal handles never escape.
pub(crate) struct SimulationJointActiveBuildOccurrence<'a> {
    authority: SimulationJointPoolAuthority,
    slot: &'a JointSlot<SimulationJointFrames, SimulationJointLimits>,
}

impl SimulationJointActiveBuildOccurrence<'_> {
    #[must_use]
    pub(crate) fn joint(&self) -> SimulationJointIdentity {
        SimulationJointHandle {
            authority: self.slot_authority(),
            id: self.slot.id,
            occurrence: self
                .slot
                .occurrence
                .expect("validated Active Joint has an occurrence"),
        }
        .identity()
    }

    #[must_use]
    pub(crate) fn body_b(&self) -> SimulationRigidBodyObservation {
        self.slot
            .body_b
            .expect("validated Active Joint has body B")
            .observation()
    }

    #[must_use]
    pub(crate) fn body_a(&self) -> SimulationRigidBodyObservation {
        self.slot
            .body_a
            .expect("validated Active Joint has body A")
            .observation()
    }

    #[must_use]
    pub(crate) fn frames(&self) -> SimulationJointFrames {
        **self
            .slot
            .frames
            .as_ref()
            .expect("validated Active Joint has frames")
    }

    pub(crate) fn try_limits(&self) -> Result<SimulationJointLimits, SimulationJointPoolError> {
        self.slot
            .limits
            .as_ref()
            .expect("validated Active Joint has limits")
            .try_read()
    }

    #[must_use]
    pub(crate) const fn spy(&self) -> u32 {
        self.slot.spy
    }

    const fn slot_authority(&self) -> SimulationJointPoolAuthority {
        self.authority
    }
}

/// Typed failure before or during one private Active-build traversal.
pub(crate) enum SimulationJointActiveBuildVisitError<E> {
    ValidationAllocationFailed { capacity: usize },
    Pool(SimulationJointPoolError),
    Visitor(E),
}

/// Exact-size iterator over a structurally validated Active right-link ring.
pub(crate) struct SimulationJointActiveBuildOccurrences<'a> {
    authority: SimulationJointPoolAuthority,
    pool: &'a SimulationJointPool<SimulationJointFrames, SimulationJointLimits>,
    next: SimulationJointNode,
    remaining: usize,
    active_ordinal: usize,
}

impl<'a> Iterator for SimulationJointActiveBuildOccurrences<'a> {
    type Item = (usize, SimulationJointActiveBuildOccurrence<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let SimulationJointNode::Joint(id) = self.next else {
            unreachable!("validated Active ring contains only Joint nodes");
        };
        let slot = &self.pool.slots[id.0 as usize];
        self.next = slot.links.right;
        self.remaining -= 1;
        let ordinal = self.active_ordinal;
        self.active_ordinal += 1;
        Some((
            ordinal,
            SimulationJointActiveBuildOccurrence {
                authority: self.authority,
                slot,
            },
        ))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for SimulationJointActiveBuildOccurrences<'_> {}

/// Typed safe-host refusal or invariant failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimulationJointPoolError {
    CapacityExceedsU32 { capacity: usize },
    AllocationFailed { capacity: usize },
    AuthorityExhausted,
    OccurrenceExhausted,
    ForeignHandle,
    SlotOutOfRange { slot: u32 },
    StaleHandle { slot: u32 },
    JointIsFree { slot: u32 },
    LimitsLockPoisoned,
    LimitsOwnerPoolMismatch,
    LimitsOwnerMismatch { slot: u32 },
    ImmutableLimits { slot: u32 },
    MutableLimitsRequireSnapshot { slot: u32 },
    CorruptStructure,
}

impl fmt::Display for SimulationJointPoolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityExceedsU32 { capacity } => {
                write!(formatter, "joint capacity {capacity} exceeds u32")
            }
            Self::AllocationFailed { capacity } => {
                write!(formatter, "could not allocate {capacity} joint slots")
            }
            Self::AuthorityExhausted => formatter.write_str("joint-pool authority exhausted"),
            Self::OccurrenceExhausted => formatter.write_str("joint occurrence exhausted"),
            Self::ForeignHandle => formatter.write_str("joint handle belongs to another pool"),
            Self::SlotOutOfRange { slot } => write!(formatter, "joint slot {slot} is out of range"),
            Self::StaleHandle { slot } => write!(formatter, "joint slot {slot} is stale"),
            Self::JointIsFree { slot } => write!(formatter, "joint slot {slot} is free"),
            Self::LimitsLockPoisoned => {
                formatter.write_str("retained Joint limits lock is poisoned")
            }
            Self::LimitsOwnerPoolMismatch => {
                formatter.write_str("mutable Joint limits belong to another pool")
            }
            Self::LimitsOwnerMismatch { slot } => write!(
                formatter,
                "joint slot {slot} retains a different limits owner"
            ),
            Self::ImmutableLimits { slot } => {
                write!(formatter, "joint slot {slot} has no mutable limits owner")
            }
            Self::MutableLimitsRequireSnapshot { slot } => write!(
                formatter,
                "joint slot {slot} requires a current mutable limits snapshot"
            ),
            Self::CorruptStructure => formatter.write_str("joint-pool structure is corrupt"),
        }
    }
}

impl Error for SimulationJointPoolError {}

/// Fixed-capacity semantic Joint pool.
#[derive(Debug)]
pub struct SimulationJointPool<F, L> {
    authority: SimulationJointPoolAuthority,
    slots: Box<[JointSlot<F, L>]>,
    anchors: [Links; 2],
    counts: SimulationJointCounts,
    next_occurrence: Option<NonZeroU64>,
}

impl<F: Copy, L: Copy> SimulationJointPool<F, L> {
    /// Creates two sentinel rings and inserts real slots into Free in ascending
    /// ordinal order. Safe host payload/tag initialization is deterministic;
    /// native `Initialize` proves only real-slot link initialization.
    ///
    /// # Errors
    ///
    /// Returns a typed error if capacity exceeds the native `u32` count, host
    /// allocation fails, or process-unique authority space is exhausted.
    pub fn try_new(capacity: usize) -> Result<Self, SimulationJointPoolError> {
        let capacity_u32 = u32::try_from(capacity)
            .map_err(|_| SimulationJointPoolError::CapacityExceedsU32 { capacity })?;
        let mut storage = Vec::new();
        storage
            .try_reserve_exact(capacity)
            .map_err(|_| SimulationJointPoolError::AllocationFailed { capacity })?;
        let authority = fresh_joint_pool_authority(&NEXT_JOINT_POOL_AUTHORITY)?;
        for index in 0..capacity_u32 {
            let id = SimulationJointId(index);
            let left = if index == 0 {
                SimulationJointNode::Anchor(SimulationJointList::Free)
            } else {
                SimulationJointNode::Joint(SimulationJointId(index - 1))
            };
            let right = if index + 1 == capacity_u32 {
                SimulationJointNode::Anchor(SimulationJointList::Free)
            } else {
                SimulationJointNode::Joint(SimulationJointId(index + 1))
            };
            storage.push(JointSlot::initialized(id, Links { right, left }));
        }

        let mut anchors = SimulationJointList::ALL.map(|list| {
            let node = SimulationJointNode::Anchor(list);
            Links {
                right: node,
                left: node,
            }
        });
        if capacity_u32 != 0 {
            anchors[SimulationJointList::Free.index()] = Links {
                right: SimulationJointNode::Joint(SimulationJointId(0)),
                left: SimulationJointNode::Joint(SimulationJointId(capacity_u32 - 1)),
            };
        }

        Ok(Self {
            authority,
            slots: storage.into_boxed_slice(),
            anchors,
            counts: SimulationJointCounts {
                free: capacity_u32,
                active: 0,
            },
            next_occurrence: NonZeroU64::new(1),
        })
    }

    #[must_use]
    pub const fn authority(&self) -> SimulationJointPoolAuthority {
        self.authority
    }

    #[must_use]
    pub const fn counts(&self) -> SimulationJointCounts {
        self.counts
    }

    /// Acquires Free head, appends Active tail, then stores the proven payload.
    /// API `parent` is deliberately written to native body B and `child` to A.
    ///
    /// `Ok(None)` is the native zero-free-count result and observes no anchor or
    /// payload. Host structural and occurrence checks precede all mutations.
    /// This pool deliberately has no body-pool reference: add stores the opaque
    /// capabilities verbatim and does not prove they are still live. A later
    /// solver integration must establish that separate semantic precondition.
    ///
    /// # Errors
    ///
    /// Returns an error without pool mutation if the ring is corrupt or the
    /// host-only occurrence space is exhausted.
    pub fn try_add_joint(
        &mut self,
        parent: SimulationRigidBodyHandle,
        child: SimulationRigidBodyHandle,
        frames: &Arc<F>,
        limits: &Arc<L>,
    ) -> Result<Option<SimulationJointHandle>, SimulationJointPoolError> {
        let Some(plan) = self.try_prepare_joint_add()? else {
            return Ok(None);
        };
        Ok(Some(self.commit_joint_add(
            plan,
            parent,
            child,
            Arc::clone(frames),
            RetainedJointLimits::Immutable(Arc::clone(limits)),
        )))
    }

    /// Same native Add graph with shared mutable storage. Native Add does NOT
    /// read limits: locking and permanent authority binding here are explicit
    /// host preflight, not native attribute/Build sampling. Empty capacity
    /// returns before even this host payload inspection.
    pub(crate) fn try_add_joint_mutable_limits(
        &mut self,
        parent: SimulationRigidBodyHandle,
        child: SimulationRigidBodyHandle,
        frames: &Arc<F>,
        limits: &SimulationMutableJointLimits<L>,
    ) -> Result<Option<SimulationJointHandle>, SimulationJointPoolError> {
        let Some(plan) = self.try_prepare_joint_add()? else {
            return Ok(None);
        };
        let mut state = limits
            .0
            .lock()
            .map_err(|_| SimulationJointPoolError::LimitsLockPoisoned)?;
        if state.pool.is_some_and(|pool| pool != self.authority) {
            return Err(SimulationJointPoolError::LimitsOwnerPoolMismatch);
        }
        let handle = self.commit_joint_add(
            plan,
            parent,
            child,
            Arc::clone(frames),
            RetainedJointLimits::Mutable(limits.clone()),
        );
        state.pool = Some(self.authority);
        Ok(Some(handle))
    }

    /// Complete Copy store into the exact existing source owner. Callers retain
    /// untouched fields explicitly; no callback runs under a lock, and no
    /// topology, endpoints, frames, tags or prepared Jacobian is changed here.
    pub(crate) fn try_write_mutable_limits(
        &mut self,
        handle: SimulationJointHandle,
        expected: &SimulationMutableJointLimits<L>,
        value: L,
    ) -> Result<(), SimulationJointPoolError> {
        let index = self.validate_handle(handle)?;
        self.validate_integrity()?;
        let Some(RetainedJointLimits::Mutable(owner)) = &self.slots[index].limits else {
            return Err(SimulationJointPoolError::ImmutableLimits { slot: handle.id.0 });
        };
        if !Arc::ptr_eq(&owner.0, &expected.0) {
            return Err(SimulationJointPoolError::LimitsOwnerMismatch { slot: handle.id.0 });
        }
        let mut state = owner
            .0
            .lock()
            .map_err(|_| SimulationJointPoolError::LimitsLockPoisoned)?;
        if state.pool != Some(self.authority) {
            return Err(SimulationJointPoolError::LimitsOwnerPoolMismatch);
        }
        state.value = value;
        Ok(())
    }

    /// Current checked limits, preserving the immutable borrowed binding API.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "checked diagnostic accessor for upcoming mutable skeleton owner"
        )
    )]
    pub(crate) fn try_limits(
        &self,
        handle: SimulationJointHandle,
    ) -> Result<L, SimulationJointPoolError> {
        let index = self.validate_handle(handle)?;
        self.validate_integrity()?;
        self.slots[index]
            .limits
            .as_ref()
            .ok_or(SimulationJointPoolError::CorruptStructure)?
            .try_read()
    }

    /// `SkeletonBody::Init` writes each admitted Joint's spy word at 0x82b621c4.
    /// This is not the adjacent tag field, and does not alter list membership.
    pub(crate) fn try_set_spy(
        &mut self,
        handle: SimulationJointHandle,
        spy: u32,
    ) -> Result<(), SimulationJointPoolError> {
        let index = self.validate_handle(handle)?;
        self.validate_integrity()?;
        self.slots[index].spy = spy;
        Ok(())
    }

    fn try_prepare_joint_add(&self) -> Result<Option<JointAddPlan>, SimulationJointPoolError> {
        if self.counts.free == 0 {
            return Ok(None);
        }
        self.validate_integrity()?;
        let occurrence = self
            .next_occurrence
            .ok_or(SimulationJointPoolError::OccurrenceExhausted)?;
        let next_occurrence = occurrence.get().checked_add(1).and_then(NonZeroU64::new);
        let free_anchor = SimulationJointNode::Anchor(SimulationJointList::Free);
        let node = self.links(free_anchor)?.right;
        let SimulationJointNode::Joint(id) = node else {
            return Err(SimulationJointPoolError::CorruptStructure);
        };
        Ok(Some(JointAddPlan {
            id,
            occurrence,
            next_occurrence,
        }))
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "consume the private one-shot preflight plan at commit"
    )]
    fn commit_joint_add(
        &mut self,
        plan: JointAddPlan,
        parent: SimulationRigidBodyHandle,
        child: SimulationRigidBodyHandle,
        retained_frames: Arc<F>,
        retained_limits: RetainedJointLimits<L>,
    ) -> SimulationJointHandle {
        let JointAddPlan {
            id,
            occurrence,
            next_occurrence,
        } = plan;
        let node = SimulationJointNode::Joint(id);
        self.unlink(node);
        self.append_to_tail(node, SimulationJointList::Active);
        let slot = &mut self.slots[id.0 as usize];
        slot.spy = 0;
        slot.frames = Some(retained_frames);
        slot.limits = Some(retained_limits);
        slot.body_b = Some(parent);
        slot.body_a = Some(child);
        self.counts.free = self.counts.free.wrapping_sub(1);
        self.counts.active = self.counts.active.wrapping_add(1);
        slot.occurrence = Some(occurrence);
        self.next_occurrence = next_occurrence;

        SimulationJointHandle {
            authority: self.authority,
            id,
            occurrence,
        }
    }

    /// Borrows the verbatim immutable binding of one active Joint. Mutable
    /// limits explicitly refuse this view rather than returning an old copy;
    /// their current value is available through the checked limits reader.
    ///
    /// # Errors
    ///
    /// Returns a typed error for a foreign, out-of-range, stale, or free handle,
    /// or when the retained Joint ring is corrupt.
    pub fn try_binding(
        &self,
        handle: SimulationJointHandle,
    ) -> Result<SimulationJointBinding<'_, F, L>, SimulationJointPoolError> {
        let index = self.validate_handle(handle)?;
        self.validate_integrity()?;
        let slot = &self.slots[index];
        let frames = slot
            .frames
            .as_ref()
            .ok_or(SimulationJointPoolError::CorruptStructure)?;
        let limits = slot
            .limits
            .as_ref()
            .ok_or(SimulationJointPoolError::CorruptStructure)?;
        let RetainedJointLimits::Immutable(limits) = limits else {
            return Err(SimulationJointPoolError::MutableLimitsRequireSnapshot {
                slot: handle.id.0,
            });
        };
        let body_a = slot
            .body_a
            .ok_or(SimulationJointPoolError::CorruptStructure)?;
        let body_b = slot
            .body_b
            .ok_or(SimulationJointPoolError::CorruptStructure)?;
        Ok(SimulationJointBinding {
            frames,
            limits,
            body_a: body_a.identity(),
            body_b: body_b.identity(),
            tag: slot.tag,
            spy: slot.spy,
        })
    }

    /// Returns an active Joint to Free tail without inspecting or validating
    /// its stored body handles and without clearing any payload field.
    ///
    /// # Errors
    ///
    /// Returns a typed error before mutation for a foreign, out-of-range, stale,
    /// or free Joint capability, or a structurally invalid Joint ring.
    pub fn try_remove_joint(
        &mut self,
        handle: SimulationJointHandle,
    ) -> Result<(), SimulationJointPoolError> {
        let index = self.validate_handle(handle)?;
        self.validate_integrity()?;
        self.remove_validated_slot(index);
        Ok(())
    }

    #[must_use]
    pub fn snapshot(&self) -> SimulationJointPoolSnapshot {
        SimulationJointPoolSnapshot {
            authority: self.authority,
            counts: self.counts,
            next_occurrence: self.next_occurrence.map(NonZeroU64::get),
            free_order: self.list_order(SimulationJointList::Free),
            active_order: self.list_order(SimulationJointList::Active),
            slots: self
                .slots
                .iter()
                .map(|slot| SimulationJointSlotSnapshot {
                    id: slot.id,
                    occurrence: slot.occurrence.map(NonZeroU64::get),
                    frames_present: slot.frames.is_some(),
                    limits_present: slot.limits.is_some(),
                    right_link: slot.links.right,
                    left_link: slot.links.left,
                    body_a: slot.body_a.map(SimulationRigidBodyHandle::identity),
                    body_b: slot.body_b.map(SimulationRigidBodyHandle::identity),
                    tag: slot.tag,
                    spy: slot.spy,
                    membership: slot.membership,
                })
                .collect(),
        }
    }

    fn validate_handle(
        &self,
        handle: SimulationJointHandle,
    ) -> Result<usize, SimulationJointPoolError> {
        if handle.authority != self.authority {
            return Err(SimulationJointPoolError::ForeignHandle);
        }
        let index = handle.id.0 as usize;
        let slot = self
            .slots
            .get(index)
            .ok_or(SimulationJointPoolError::SlotOutOfRange { slot: handle.id.0 })?;
        if slot.occurrence != Some(handle.occurrence) {
            return Err(SimulationJointPoolError::StaleHandle { slot: handle.id.0 });
        }
        if slot.membership == SimulationJointList::Free {
            return Err(SimulationJointPoolError::JointIsFree { slot: handle.id.0 });
        }
        Ok(index)
    }

    fn validate_identity(
        &self,
        identity: SimulationJointIdentity,
    ) -> Result<usize, SimulationJointPoolError> {
        if identity.authority != self.authority {
            return Err(SimulationJointPoolError::ForeignHandle);
        }
        let index = identity.id.0 as usize;
        let slot = self
            .slots
            .get(index)
            .ok_or(SimulationJointPoolError::SlotOutOfRange {
                slot: identity.id.0,
            })?;
        if slot.occurrence != Some(identity.occurrence) {
            return Err(SimulationJointPoolError::StaleHandle {
                slot: identity.id.0,
            });
        }
        if slot.membership == SimulationJointList::Free {
            return Err(SimulationJointPoolError::JointIsFree {
                slot: identity.id.0,
            });
        }
        Ok(index)
    }

    fn remove_validated_slot(&mut self, index: usize) {
        let node = SimulationJointNode::Joint(self.slots[index].id);
        self.unlink(node);
        self.append_to_tail(node, SimulationJointList::Free);
        self.counts.active = self.counts.active.wrapping_sub(1);
        self.counts.free = self.counts.free.wrapping_add(1);
    }

    fn unlink(&mut self, node: SimulationJointNode) {
        let links = self.links(node).expect("validated Joint node has links");
        self.links_mut(links.right)
            .expect("validated right neighbor")
            .left = links.left;
        self.links_mut(links.left)
            .expect("validated left neighbor")
            .right = links.right;
    }

    fn append_to_tail(&mut self, node: SimulationJointNode, list: SimulationJointList) {
        let anchor = SimulationJointNode::Anchor(list);
        let tail = self.links(anchor).expect("Joint anchor exists").left;
        {
            let links = self.links_mut(node).expect("validated Joint node");
            links.right = anchor;
            links.left = tail;
        }
        self.links_mut(tail).expect("validated Joint tail").right = node;
        self.links_mut(anchor).expect("Joint anchor exists").left = node;
        if let SimulationJointNode::Joint(id) = node {
            self.slots[id.0 as usize].membership = list;
        }
    }

    fn links(&self, node: SimulationJointNode) -> Result<Links, SimulationJointPoolError> {
        match node {
            SimulationJointNode::Anchor(list) => Ok(self.anchors[list.index()]),
            SimulationJointNode::Joint(id) => self
                .slots
                .get(id.0 as usize)
                .map(|slot| slot.links)
                .ok_or(SimulationJointPoolError::CorruptStructure),
        }
    }

    fn links_mut(&mut self, node: SimulationJointNode) -> Option<&mut Links> {
        match node {
            SimulationJointNode::Anchor(list) => self.anchors.get_mut(list.index()),
            SimulationJointNode::Joint(id) => self
                .slots
                .get_mut(id.0 as usize)
                .map(|slot| &mut slot.links),
        }
    }

    fn validate_integrity(&self) -> Result<(), SimulationJointPoolError> {
        let mut seen = vec![false; self.slots.len()];
        self.validate_integrity_with_seen(&mut seen)
    }

    fn validate_integrity_with_seen(
        &self,
        seen: &mut [bool],
    ) -> Result<(), SimulationJointPoolError> {
        if seen.len() != self.slots.len() || seen.iter().any(|present| *present) {
            return Err(SimulationJointPoolError::CorruptStructure);
        }
        for list in SimulationJointList::ALL {
            let anchor = SimulationJointNode::Anchor(list);
            let anchor_links = self.links(anchor)?;
            if self.links(anchor_links.right)?.left != anchor
                || self.links(anchor_links.left)?.right != anchor
            {
                return Err(SimulationJointPoolError::CorruptStructure);
            }
            let mut node = anchor_links.right;
            let mut count = 0_u32;
            while node != anchor {
                let SimulationJointNode::Joint(id) = node else {
                    return Err(SimulationJointPoolError::CorruptStructure);
                };
                let index = id.0 as usize;
                let Some(slot) = self.slots.get(index) else {
                    return Err(SimulationJointPoolError::CorruptStructure);
                };
                if seen[index]
                    || slot.id != id
                    || slot.membership != list
                    || (list == SimulationJointList::Active
                        && (slot.occurrence.is_none()
                            || slot.frames.is_none()
                            || slot.limits.is_none()
                            || slot.body_a.is_none()
                            || slot.body_b.is_none()))
                {
                    return Err(SimulationJointPoolError::CorruptStructure);
                }
                let links = slot.links;
                if self.links(links.right)?.left != node || self.links(links.left)?.right != node {
                    return Err(SimulationJointPoolError::CorruptStructure);
                }
                seen[index] = true;
                count = count.wrapping_add(1);
                if count as usize > self.slots.len() {
                    return Err(SimulationJointPoolError::CorruptStructure);
                }
                node = links.right;
            }
            if count != self.counts.get(list) {
                return Err(SimulationJointPoolError::CorruptStructure);
            }
        }
        if seen.iter().any(|present| !*present) {
            return Err(SimulationJointPoolError::CorruptStructure);
        }
        Ok(())
    }

    fn list_order(&self, list: SimulationJointList) -> Vec<SimulationJointId> {
        let anchor = SimulationJointNode::Anchor(list);
        let mut node = self.anchors[list.index()].right;
        let mut result = Vec::new();
        while node != anchor && result.len() < self.slots.len() {
            let SimulationJointNode::Joint(id) = node else {
                break;
            };
            result.push(id);
            let Some(slot) = self.slots.get(id.0 as usize) else {
                break;
            };
            node = slot.links.right;
        }
        result
    }
}

impl SimulationJointPool<SimulationJointFrames, SimulationJointLimits> {
    /// Revalidates prepared Joint occurrences and exact semantic endpoints.
    ///
    /// The caller retains an immutable pool borrow acquired only after full
    /// structure validation, so this pass is deliberately nonallocating. Body
    /// occurrence liveness remains the exclusive RF phase's responsibility.
    pub(crate) fn try_validate_build_bindings(
        &self,
        bindings: impl IntoIterator<Item = SimulationJointBuildBinding>,
    ) -> Result<(), SimulationJointBuildBindingValidationError> {
        for (ordinal, binding) in bindings.into_iter().enumerate() {
            let map_binding_error = |source| SimulationJointBuildBindingValidationError::Binding {
                ordinal,
                joint: binding.joint,
                source,
            };
            let index = self
                .validate_identity(binding.joint)
                .map_err(map_binding_error)?;
            let slot = &self.slots[index];
            let body_b = slot
                .body_b
                .ok_or(SimulationJointPoolError::CorruptStructure)
                .map_err(map_binding_error)?;
            if body_b.observation() != binding.body_b {
                return Err(
                    SimulationJointBuildBindingValidationError::EndpointMismatch {
                        ordinal,
                        joint: binding.joint,
                        endpoint: SimulationJointBuildEndpoint::Parent,
                    },
                );
            }
            let body_a = slot
                .body_a
                .ok_or(SimulationJointPoolError::CorruptStructure)
                .map_err(map_binding_error)?;
            if body_a.observation() != binding.body_a {
                return Err(
                    SimulationJointBuildBindingValidationError::EndpointMismatch {
                        ordinal,
                        joint: binding.joint,
                        endpoint: SimulationJointBuildEndpoint::Child,
                    },
                );
            }
        }
        Ok(())
    }

    /// Lends exact-payload Active occurrences in native right-link order.
    ///
    /// The complete Joint structure is validated with fallibly reserved
    /// scratch before the inspection closure. The closure can therefore
    /// reserve all later staging before it reads the first endpoint, while the
    /// iterator's accessors preserve B-then-A state admission and skipped
    /// payloads.
    pub(crate) fn try_with_active_build_occurrences_and_reservation<R, E>(
        &self,
        inspect: impl FnOnce(SimulationJointActiveBuildOccurrences<'_>) -> Result<R, E>,
        reserve: impl FnOnce(&mut Vec<bool>, usize) -> Result<(), ()>,
    ) -> Result<R, SimulationJointActiveBuildVisitError<E>> {
        let capacity = self.slots.len();
        let mut seen = Vec::new();
        reserve(&mut seen, capacity).map_err(|()| {
            SimulationJointActiveBuildVisitError::ValidationAllocationFailed { capacity }
        })?;
        if seen.capacity() < capacity {
            return Err(
                SimulationJointActiveBuildVisitError::ValidationAllocationFailed { capacity },
            );
        }
        seen.resize(capacity, false);
        self.validate_integrity_with_seen(&mut seen)
            .map_err(SimulationJointActiveBuildVisitError::Pool)?;

        inspect(SimulationJointActiveBuildOccurrences {
            authority: self.authority,
            pool: self,
            next: self.anchors[SimulationJointList::Active.index()].right,
            remaining: self.counts.active as usize,
            active_ordinal: 0,
        })
        .map_err(SimulationJointActiveBuildVisitError::Visitor)
    }
}

fn fresh_joint_pool_authority(
    source: &AtomicU64,
) -> Result<SimulationJointPoolAuthority, SimulationJointPoolError> {
    source
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .map(SimulationJointPoolAuthority)
        .map_err(|_| SimulationJointPoolError::AuthorityExhausted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation_rigid_body_pool::SimulationRigidBodyPool;
    use std::mem::{align_of, offset_of, size_of};

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct Frames(u32);

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct Limits(u32);

    fn values(words: [u32; 4]) -> [f32; 4] {
        words.map(f32::from_bits)
    }

    fn bits(value: [f32; 4]) -> [u32; 4] {
        value.map(f32::to_bits)
    }

    #[test]
    fn native_frame_and_limit_layouts_match_pdb() {
        assert_eq!(size_of::<SimulationJointFrames>(), 0x50);
        assert_eq!(align_of::<SimulationJointFrames>(), 0x10);
        assert_eq!(
            [
                offset_of!(SimulationJointFrames, quaternion_a),
                offset_of!(SimulationJointFrames, position_a),
                offset_of!(SimulationJointFrames, quaternion_b),
                offset_of!(SimulationJointFrames, position_b),
                offset_of!(SimulationJointFrames, quaternion_l),
            ],
            [0x00, 0x10, 0x20, 0x30, 0x40]
        );

        assert_eq!(size_of::<SimulationJointLimits>(), 0x40);
        assert_eq!(align_of::<SimulationJointLimits>(), 0x10);
        assert_eq!(
            [
                offset_of!(SimulationJointLimits, p_prism),
                offset_of!(SimulationJointLimits, v_prism),
                offset_of!(SimulationJointLimits, v_twist),
                offset_of!(SimulationJointLimits, v_swing),
                offset_of!(SimulationJointLimits, swing_a),
                offset_of!(SimulationJointLimits, twist_a),
                offset_of!(SimulationJointLimits, swing_c),
                offset_of!(SimulationJointLimits, twist_c),
                offset_of!(SimulationJointLimits, raw_swing_type),
                offset_of!(SimulationJointLimits, raw_twist_type),
            ],
            [0x00, 0x10, 0x20, 0x24, 0x28, 0x2c, 0x30, 0x34, 0x38, 0x3c]
        );
    }

    #[test]
    fn native_frame_and_limit_words_round_trip_without_enum_normalization() {
        let frames = SimulationJointFrames {
            quaternion_a: values([0x3f80_0001, 0xbf00_0002, 0x4000_0003, 0xc040_0004]),
            position_a: values([0x4080_0005, 0xc0a0_0006, 0x40c0_0007, 0xc0e0_0008]),
            quaternion_b: values([0x4100_0009, 0xc110_000a, 0x4120_000b, 0xc130_000c]),
            position_b: values([0x4140_000d, 0xc150_000e, 0x4160_000f, 0xc170_0010]),
            quaternion_l: values([0x4180_0011, 0xc188_0012, 0x4190_0013, 0xc198_0014]),
        };
        assert_eq!(
            [
                bits(frames.quaternion_a),
                bits(frames.position_a),
                bits(frames.quaternion_b),
                bits(frames.position_b),
                bits(frames.quaternion_l),
            ],
            [
                [0x3f80_0001, 0xbf00_0002, 0x4000_0003, 0xc040_0004],
                [0x4080_0005, 0xc0a0_0006, 0x40c0_0007, 0xc0e0_0008],
                [0x4100_0009, 0xc110_000a, 0x4120_000b, 0xc130_000c],
                [0x4140_000d, 0xc150_000e, 0x4160_000f, 0xc170_0010],
                [0x4180_0011, 0xc188_0012, 0x4190_0013, 0xc198_0014],
            ]
        );

        let limits = SimulationJointLimits {
            p_prism: values([0x3e80_0021, 0xbe90_0022, 0x3ea0_0023, 0xbeb0_0024]),
            v_prism: values([0x3ec0_0025, 0xbed0_0026, 0x3ee0_0027, 0xbef0_0028]),
            v_twist: f32::from_bits(0x3f00_0029),
            v_swing: f32::from_bits(0xbf08_002a),
            swing_a: f32::from_bits(0x3f10_002b),
            twist_a: f32::from_bits(0xbf18_002c),
            swing_c: f32::from_bits(0x3f20_002d),
            twist_c: f32::from_bits(0xbf28_002e),
            raw_swing_type: 0xffff_ff80,
            raw_twist_type: 0x8000_0004,
        };
        assert_eq!(
            bits(limits.p_prism),
            [0x3e80_0021, 0xbe90_0022, 0x3ea0_0023, 0xbeb0_0024]
        );
        assert_eq!(
            bits(limits.v_prism),
            [0x3ec0_0025, 0xbed0_0026, 0x3ee0_0027, 0xbef0_0028]
        );
        assert_eq!(
            [
                limits.v_twist.to_bits(),
                limits.v_swing.to_bits(),
                limits.swing_a.to_bits(),
                limits.twist_a.to_bits(),
                limits.swing_c.to_bits(),
                limits.twist_c.to_bits(),
                limits.raw_swing_type,
                limits.raw_twist_type,
            ],
            [
                0x3f00_0029,
                0xbf08_002a,
                0x3f10_002b,
                0xbf18_002c,
                0x3f20_002d,
                0xbf28_002e,
                0xffff_ff80,
                0x8000_0004,
            ]
        );
    }

    fn bodies(count: usize) -> (SimulationRigidBodyPool, Vec<SimulationRigidBodyHandle>) {
        let mut pool = SimulationRigidBodyPool::try_new(count).expect("body pool");
        pool.set_gravity([0.0; 3]);
        pool.set_cooldown(0);
        let handles = (0..count)
            .map(|_| {
                pool.try_add_rigid_body(
                    [
                        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0,
                        1.0,
                    ],
                    None,
                    4,
                )
                .expect("body pool")
                .expect("body")
            })
            .collect();
        (pool, handles)
    }

    fn add(
        pool: &mut SimulationJointPool<Frames, Limits>,
        parent: SimulationRigidBodyHandle,
        child: SimulationRigidBodyHandle,
        value: u32,
    ) -> (SimulationJointHandle, Arc<Frames>, Arc<Limits>) {
        let frames = Arc::new(Frames(value));
        let limits = Arc::new(Limits(value + 100));
        let handle = pool
            .try_add_joint(parent, child, &frames, &limits)
            .expect("valid Joint pool")
            .expect("free Joint");
        (handle, frames, limits)
    }

    #[test]
    fn initialization_has_two_sentinels_ascending_slots_and_atomic_exhaustion() {
        let (_, body) = bodies(2);
        let mut pool = SimulationJointPool::<Frames, Limits>::try_new(2).expect("Joint pool");
        let initial = pool.snapshot();
        assert_eq!(initial.counts, SimulationJointCounts { free: 2, active: 0 });
        assert_eq!(
            initial.free_order,
            [SimulationJointId(0), SimulationJointId(1)]
        );
        assert!(initial.active_order.is_empty());
        assert!(initial.slots.iter().all(|slot| {
            !slot.frames_present
                && !slot.limits_present
                && slot.body_a.is_none()
                && slot.body_b.is_none()
                && slot.tag == 0
                && slot.spy == 0
        }));
        add(&mut pool, body[0], body[1], 1);
        add(&mut pool, body[1], body[0], 2);
        let full = pool.snapshot();
        let frames = Arc::new(Frames(3));
        let limits = Arc::new(Limits(103));
        assert_eq!(
            pool.try_add_joint(body[0], body[1], &frames, &limits)
                .expect("full is ordinary"),
            None
        );
        assert_eq!(pool.snapshot(), full);
        assert_eq!(Arc::strong_count(&frames), 1);
        assert_eq!(Arc::strong_count(&limits), 1);

        let mut zero = SimulationJointPool::<Frames, Limits>::try_new(0).expect("zero pool");
        assert_eq!(
            zero.try_add_joint(body[0], body[1], &frames, &limits)
                .expect("zero is ordinary"),
            None
        );
    }

    #[test]
    fn add_maps_parent_to_body_b_child_to_body_a_and_retains_payload_identity() {
        let (_, body) = bodies(2);
        let mut pool = SimulationJointPool::try_new(1).expect("Joint pool");
        pool.slots[0].tag = 0x1122_3344;
        pool.slots[0].spy = u32::MAX;
        let (handle, frames, limits) = add(&mut pool, body[0], body[1], 7);
        let binding = pool.try_binding(handle).expect("binding");
        assert_eq!(binding.body_a, body[1].identity());
        assert_eq!(binding.body_b, body[0].identity());
        assert!(Arc::ptr_eq(binding.frames, &frames));
        assert!(Arc::ptr_eq(binding.limits, &limits));
        assert_eq!(binding.tag, 0x1122_3344);
        assert_eq!(binding.spy, 0);
        assert_eq!(Arc::strong_count(&frames), 2);
        assert_eq!(Arc::strong_count(&limits), 2);
    }

    #[test]
    fn active_order_and_middle_removal_free_fifo_match_oracle() {
        let (_, body) = bodies(2);
        let mut pool = SimulationJointPool::try_new(4).expect("Joint pool");
        let (zero, _, _) = add(&mut pool, body[0], body[1], 0);
        let (one, _, _) = add(&mut pool, body[0], body[1], 1);
        let (two, _, _) = add(&mut pool, body[0], body[1], 2);
        assert_eq!(
            pool.snapshot().active_order,
            [zero.id(), one.id(), two.id()]
        );
        pool.try_remove_joint(one).expect("middle removal");
        assert_eq!(pool.snapshot().active_order, [zero.id(), two.id()]);
        assert_eq!(pool.snapshot().free_order, [SimulationJointId(3), one.id()]);
        let (three, _, _) = add(&mut pool, body[0], body[1], 3);
        assert_eq!(three.id(), SimulationJointId(3));
        let (reused_one, _, _) = add(&mut pool, body[0], body[1], 4);
        assert_eq!(reused_one.id(), one.id());
    }

    #[test]
    fn removal_after_bodies_are_removed_never_validates_or_dereferences_them() {
        let (mut body_pool, body) = bodies(2);
        let mut pool = SimulationJointPool::try_new(1).expect("Joint pool");
        let (joint, frames, limits) = add(&mut pool, body[0], body[1], 5);
        body_pool
            .try_remove_rigid_body(body[0])
            .expect("parent body first");
        body_pool
            .try_remove_rigid_body(body[1])
            .expect("child body first");

        pool.try_remove_joint(joint)
            .expect("Joint removal ignores stale body capabilities");
        let slot = pool.snapshot().slots[0];
        assert_eq!(slot.body_a, Some(body[1].identity()));
        assert_eq!(slot.body_b, Some(body[0].identity()));
        assert!(slot.frames_present);
        assert!(slot.limits_present);
        assert_eq!(Arc::strong_count(&frames), 2);
        assert_eq!(Arc::strong_count(&limits), 2);
    }

    #[test]
    fn removal_preserves_payload_spy_tag_and_bodies_until_reuse() {
        let (_, body) = bodies(2);
        let mut pool = SimulationJointPool::try_new(1).expect("Joint pool");
        pool.slots[0].tag = 0xaabb_ccdd;
        let (old_handle, old_frames, old_limits) = add(&mut pool, body[0], body[1], 6);
        pool.slots[0].spy = 0x5566_7788;
        let before = pool.snapshot().slots[0];
        pool.try_remove_joint(old_handle).expect("remove");
        let free = pool.snapshot().slots[0];
        assert_eq!(free.occurrence, before.occurrence);
        assert_eq!(free.body_a, before.body_a);
        assert_eq!(free.body_b, before.body_b);
        assert_eq!(free.tag, 0xaabb_ccdd);
        assert_eq!(free.spy, 0x5566_7788);
        assert_eq!(Arc::strong_count(&old_frames), 2);
        assert_eq!(Arc::strong_count(&old_limits), 2);

        let (fresh, new_frames, new_limits) = add(&mut pool, body[1], body[0], 9);
        let binding = pool.try_binding(fresh).expect("binding");
        assert_eq!(binding.body_a, body[0].identity());
        assert_eq!(binding.body_b, body[1].identity());
        assert_eq!(binding.spy, 0);
        assert_eq!(binding.tag, 0xaabb_ccdd);
        assert!(Arc::ptr_eq(binding.frames, &new_frames));
        assert!(Arc::ptr_eq(binding.limits, &new_limits));
        assert_eq!(Arc::strong_count(&old_frames), 1);
        assert_eq!(Arc::strong_count(&old_limits), 1);
    }

    #[test]
    fn foreign_double_and_reused_slot_stale_handles_are_atomic() {
        let (_, body) = bodies(2);
        let mut first = SimulationJointPool::try_new(1).expect("Joint pool");
        let mut second = SimulationJointPool::<Frames, Limits>::try_new(1).expect("Joint pool");
        let (old, _, _) = add(&mut first, body[0], body[1], 1);
        let second_before = second.snapshot();
        assert_eq!(
            second.try_remove_joint(old),
            Err(SimulationJointPoolError::ForeignHandle)
        );
        assert_eq!(second.snapshot(), second_before);

        first.try_remove_joint(old).expect("remove");
        let removed = first.snapshot();
        assert_eq!(
            first.try_remove_joint(old),
            Err(SimulationJointPoolError::JointIsFree { slot: 0 })
        );
        assert_eq!(first.snapshot(), removed);
        let (fresh, _, _) = add(&mut first, body[0], body[1], 2);
        let reused = first.snapshot();
        assert_eq!(
            first.try_remove_joint(old),
            Err(SimulationJointPoolError::StaleHandle { slot: 0 })
        );
        assert_eq!(first.snapshot(), reused);
        first.try_remove_joint(fresh).expect("fresh handle");
    }

    #[test]
    fn final_occurrence_is_issued_once_then_exhaustion_is_atomic() {
        let (_, body) = bodies(2);
        let mut pool = SimulationJointPool::try_new(1).expect("Joint pool");
        pool.next_occurrence = NonZeroU64::new(u64::MAX);
        let (last, _, _) = add(&mut pool, body[0], body[1], 1);
        assert_eq!(last.occurrence.get(), u64::MAX);
        pool.try_remove_joint(last).expect("remove");
        let before = pool.snapshot();
        let frames = Arc::new(Frames(2));
        let limits = Arc::new(Limits(102));
        assert_eq!(
            pool.try_add_joint(body[0], body[1], &frames, &limits),
            Err(SimulationJointPoolError::OccurrenceExhausted)
        );
        assert_eq!(pool.snapshot(), before);
        assert_eq!(Arc::strong_count(&frames), 1);
        assert_eq!(Arc::strong_count(&limits), 1);
    }

    #[test]
    fn corruption_rejects_add_and_remove_before_effects() {
        let (_, body) = bodies(2);
        let mut add_pool = SimulationJointPool::<Frames, Limits>::try_new(2).expect("Joint pool");
        add_pool.slots[0].links.right = SimulationJointNode::Joint(SimulationJointId(0));
        let before = add_pool.snapshot();
        let frames = Arc::new(Frames(1));
        let limits = Arc::new(Limits(101));
        assert_eq!(
            add_pool.try_add_joint(body[0], body[1], &frames, &limits),
            Err(SimulationJointPoolError::CorruptStructure)
        );
        assert_eq!(add_pool.snapshot(), before);
        assert_eq!(Arc::strong_count(&frames), 1);
        assert_eq!(Arc::strong_count(&limits), 1);

        let mut remove_pool = SimulationJointPool::try_new(2).expect("Joint pool");
        let (joint, _, _) = add(&mut remove_pool, body[0], body[1], 2);
        remove_pool.slots[1].links.left = SimulationJointNode::Joint(joint.id());
        let before = remove_pool.snapshot();
        assert_eq!(
            remove_pool.try_remove_joint(joint),
            Err(SimulationJointPoolError::CorruptStructure)
        );
        assert_eq!(remove_pool.snapshot(), before);
    }

    #[test]
    fn count_updates_wrap_in_the_native_internal_primitives() {
        let (_, body) = bodies(2);
        let mut pool = SimulationJointPool::try_new(1).expect("Joint pool");
        let (joint, _, _) = add(&mut pool, body[0], body[1], 1);
        pool.counts.active = 0;
        pool.counts.free = u32::MAX;
        pool.remove_validated_slot(0);
        assert_eq!(pool.counts.active, u32::MAX);
        assert_eq!(pool.counts.free, 0);
        assert_eq!(pool.slots[0].occurrence, Some(joint.occurrence));
    }

    #[test]
    fn local_authority_allocator_is_checked_and_never_wraps() {
        let source = AtomicU64::new(u64::MAX - 1);
        assert_eq!(
            fresh_joint_pool_authority(&source),
            Ok(SimulationJointPoolAuthority(u64::MAX - 1))
        );
        assert_eq!(
            fresh_joint_pool_authority(&source),
            Err(SimulationJointPoolError::AuthorityExhausted)
        );
        assert_eq!(source.load(Ordering::Relaxed), u64::MAX);
    }
}
