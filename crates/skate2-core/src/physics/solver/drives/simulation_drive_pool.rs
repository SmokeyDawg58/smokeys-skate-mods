//! Safe semantic model of the preallocated `rw::physics::Simulation` Drive pool.
//!
//! `Simulation::Initialize` (`0x82ae7d88..0x82ae7e14`) constructs Free and
//! Active circular lists with `0x20`-byte slots in ascending order.
//! `Simulation::AddDrive` (`0x82ae43f0..0x82ae4484`) takes the Free head and
//! appends it to the Active tail; `Simulation::RemoveDrive`
//! (`0x82ae3360..0x82ae33b4`) appends a removed slot to the Free tail. This
//! module preserves that FIFO behavior with indices rather than pointers.
//!
//! Evidence packet:
//! `simulation_drive_pool_evidence.md`, SHA-256
//! `AE4AA3787723D74C1CD46871029DFF56E206045972A70BEF36FBE686999744E5`.
//!
//! Native Drive slots retain nonowning `DriveFrames *` and `DriveDynamics *`.
//! The host retains `Arc<RwLock<_>>` owners while those semantic
//! pointers remain in a slot, including after removal. This both keeps the
//! pointed-to values alive and lets later writes remain visible to the solver,
//! as native pointer retention requires. Locking, synchronization, and poison
//! reporting are safe-host policy; Add and Remove never acquire these locks.
//! Handles carry process-unique pool authority and non-reused occurrences;
//! native code has neither validation nor ABA protection.
//!
//! Pool initialization and `AddDrive` never write native `m_tag`. Its first-use
//! bits are therefore uninitialized at this evidence boundary. The safe host
//! represents that state as `None`, not an invented zero, and preserves an
//! explicitly installed tag through removal/reuse. `AddDrive` always writes
//! `m_spy=0`.

use crate::simulation_rigid_body_pool::{
    SimulationRigidBodyDriveBuildSnapshot, SimulationRigidBodyDynamicUpdatePhase,
    SimulationRigidBodyHandle, SimulationRigidBodyIdentity, SimulationRigidBodyObservation,
    SimulationRigidBodyPool, SimulationRigidBodyPoolAuthority, SimulationRigidBodyPoolError,
};
use std::{
    error::Error,
    fmt,
    num::NonZeroU64,
    sync::{
        Arc, OnceLock, RwLock,
        atomic::{AtomicU64, Ordering},
    },
};

const ACTIVE_BODY_BIT: u32 = 4;

#[path = "simulation_drive_normalization.rs"]
pub(crate) mod normalization;

static NEXT_DRIVE_POOL_AUTHORITY: AtomicU64 = AtomicU64::new(1);

/// Exact four-vector payload of native `DriveFrames` (`sizeof=0x40`).
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimulationDriveFrames {
    pub quaternion_a: [f32; 4],
    pub position_a: [f32; 4],
    pub quaternion_b: [f32; 4],
    pub position_b: [f32; 4],
}

/// Exact named fields of one native `DriveDynamics::Params` (`sizeof=0x10`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimulationDriveDynamicsParams {
    pub spring: f32,
    pub damping: f32,
    pub strength: f32,
    /// Raw `DriveType`: native Build compares exactly against 1 and 2.
    pub raw_type: u32,
}

/// Exact two-parameter payload of native `DriveDynamics` (`sizeof=0x20`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimulationDriveDynamics {
    pub linear: SimulationDriveDynamicsParams,
    pub angular: SimulationDriveDynamicsParams,
}

/// Opaque identity of one Drive pool instance.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SimulationDrivePoolAuthority(u64);

/// Stable host identity of one preallocated Drive slot.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SimulationDriveId(u32);

impl SimulationDriveId {
    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }
}

/// Copy-only capability naming one allocation occurrence of one Drive slot.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SimulationDriveHandle {
    authority: SimulationDrivePoolAuthority,
    id: SimulationDriveId,
    occurrence: NonZeroU64,
}

impl SimulationDriveHandle {
    #[must_use]
    pub const fn id(self) -> SimulationDriveId {
        self.id
    }

    /// Removes mutation authority while retaining ABA-safe occurrence identity.
    #[must_use]
    pub const fn identity(self) -> SimulationDriveIdentity {
        SimulationDriveIdentity {
            authority: self.authority,
            id: self.id,
            occurrence: self.occurrence,
        }
    }
}

/// Occurrence-bound read identity for one allocated Drive.
///
/// This type is deliberately not accepted by removal or any mutating API.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SimulationDriveIdentity {
    authority: SimulationDrivePoolAuthority,
    id: SimulationDriveId,
    occurrence: NonZeroU64,
}

impl SimulationDriveIdentity {
    #[must_use]
    pub const fn id(self) -> SimulationDriveId {
        self.id
    }
}

/// One of the two native intrusive Drive rings.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SimulationDriveList {
    Free,
    Active,
}

impl SimulationDriveList {
    const ALL: [Self; 2] = [Self::Free, Self::Active];

    const fn index(self) -> usize {
        match self {
            Self::Free => 0,
            Self::Active => 1,
        }
    }
}

/// A Drive link target exposed through read-only snapshots.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationDriveNode {
    Anchor(SimulationDriveList),
    Drive(SimulationDriveId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Links {
    right: SimulationDriveNode,
    left: SimulationDriveNode,
}

/// Native-width Drive-list counts.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SimulationDriveCounts {
    pub free: u32,
    pub active: u32,
}

impl SimulationDriveCounts {
    const fn get(self, list: SimulationDriveList) -> u32 {
        match list {
            SimulationDriveList::Free => self.free,
            SimulationDriveList::Active => self.active,
        }
    }
}

#[derive(Debug)]
struct DriveSlot {
    id: SimulationDriveId,
    occurrence: Option<NonZeroU64>,
    frames: Option<Arc<RwLock<SimulationDriveFrames>>>,
    dynamics: Option<Arc<RwLock<SimulationDriveDynamics>>>,
    links: Links,
    /// Native `m_bodyA`: semantic child.
    body_a: Option<SimulationRigidBodyHandle>,
    /// Native `m_bodyB`: semantic parent.
    body_b: Option<SimulationRigidBodyHandle>,
    tag: Option<u32>,
    spy: Option<u32>,
    membership: SimulationDriveList,
}

impl DriveSlot {
    fn initialized(id: SimulationDriveId, links: Links) -> Self {
        Self {
            id,
            occurrence: None,
            frames: None,
            dynamics: None,
            links,
            body_a: None,
            body_b: None,
            tag: None,
            spy: None,
            membership: SimulationDriveList::Free,
        }
    }
}

/// Which semantic endpoint could not be resolved for a Build-order query.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationDriveEndpoint {
    Parent,
    Child,
}

/// One admitted Drive and the detached values needed by Jacobian Build.
///
/// The body observations are retained separately from the copied numeric
/// values so a later solver coordinator can revalidate both allocation
/// occurrences before mutating either private RF row. This crate-private value
/// owns no lock guard, `Arc`, removal handle, or mutable pool capability.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "reserved for the private Drive Jacobian batch seam"
    )
)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SimulationDriveBuildSnapshot {
    pub(crate) drive: SimulationDriveIdentity,
    pub(crate) body_a: SimulationRigidBodyDriveBuildSnapshot,
    pub(crate) body_b: SimulationRigidBodyDriveBuildSnapshot,
    pub(crate) frames: SimulationDriveFrames,
    pub(crate) dynamics: SimulationDriveDynamics,
    pub(crate) time_step: f32,
    pub(crate) spy: u32,
}

/// Occurrence-qualified endpoints expected by one prepared Drive record.
///
/// This copied value grants no removal, payload-lock, or RF-row capability.
/// It exists only so the private solver boundary can prove that a prepared
/// record still names the same live Drive occurrence and semantic endpoints.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SimulationDriveBuildBinding {
    drive: SimulationDriveIdentity,
    body_a: SimulationRigidBodyObservation,
    body_b: SimulationRigidBodyObservation,
}

impl SimulationDriveBuildBinding {
    #[must_use]
    pub(crate) const fn new(
        drive: SimulationDriveIdentity,
        body_a: SimulationRigidBodyObservation,
        body_b: SimulationRigidBodyObservation,
    ) -> Self {
        Self {
            drive,
            body_a,
            body_b,
        }
    }
}

/// Attribution for one no-lock prepared-binding validation pass.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SimulationDriveBuildBindingValidationError {
    Pool(SimulationDrivePoolError),
    Binding {
        ordinal: usize,
        drive: SimulationDriveIdentity,
        source: SimulationDrivePoolError,
    },
    EndpointMismatch {
        ordinal: usize,
        drive: SimulationDriveIdentity,
        endpoint: SimulationDriveEndpoint,
    },
}

/// Read-only semantic snapshot of one Drive slot.
#[derive(Clone, Debug, PartialEq)]
pub struct SimulationDriveSlotSnapshot {
    pub id: SimulationDriveId,
    pub occurrence: Option<u64>,
    /// Whether this slot retains a frames owner; snapshots never lock it.
    pub frames_present: bool,
    /// Whether this slot retains a dynamics owner; snapshots never lock it.
    pub dynamics_present: bool,
    pub right_link: SimulationDriveNode,
    pub left_link: SimulationDriveNode,
    /// Inert pool-qualified identity for native `m_bodyA`, the semantic child.
    pub body_a: Option<SimulationRigidBodyIdentity>,
    /// Inert pool-qualified identity for native `m_bodyB`, the semantic parent.
    pub body_b: Option<SimulationRigidBodyIdentity>,
    /// `None` is the safe representation of never-initialized native bits.
    pub tag: Option<u32>,
    /// `None` before first allocation; `AddDrive` installs `Some(0)`.
    pub spy: Option<u32>,
    pub membership: SimulationDriveList,
}

/// Complete read-only Drive-pool snapshot, including exact ring order.
#[derive(Clone, Debug, PartialEq)]
pub struct SimulationDrivePoolSnapshot {
    pub authority: SimulationDrivePoolAuthority,
    pub body_pool_authority: SimulationRigidBodyPoolAuthority,
    pub counts: SimulationDriveCounts,
    pub next_occurrence: Option<u64>,
    pub free_order: Vec<SimulationDriveId>,
    pub active_order: Vec<SimulationDriveId>,
    pub slots: Vec<SimulationDriveSlotSnapshot>,
}

/// Typed safe-host refusal or invariant failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimulationDrivePoolError {
    CapacityExceedsU32 {
        capacity: usize,
    },
    AllocationFailed {
        capacity: usize,
    },
    AuthorityExhausted,
    OccurrenceExhausted,
    ForeignHandle,
    SlotOutOfRange {
        slot: u32,
    },
    StaleHandle {
        slot: u32,
    },
    DriveIsFree {
        slot: u32,
    },
    BodyPoolMismatch,
    BodyAccess {
        drive: Option<u32>,
        endpoint: SimulationDriveEndpoint,
        source: SimulationRigidBodyPoolError,
    },
    FramesPoisoned {
        slot: u32,
    },
    DynamicsPoisoned {
        slot: u32,
    },
    EndpointMismatch {
        slot: u32,
        endpoint: SimulationDriveEndpoint,
    },
    FramesOwnerMismatch {
        slot: u32,
    },
    DynamicsOwnerMismatch {
        slot: u32,
    },
    CorruptStructure,
}

impl fmt::Display for SimulationDrivePoolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityExceedsU32 { capacity } => {
                write!(formatter, "Drive capacity {capacity} exceeds u32")
            }
            Self::AllocationFailed { capacity } => {
                write!(formatter, "could not allocate {capacity} Drive slots")
            }
            Self::AuthorityExhausted => formatter.write_str("Drive-pool authority exhausted"),
            Self::OccurrenceExhausted => formatter.write_str("Drive occurrence exhausted"),
            Self::ForeignHandle => formatter.write_str("Drive handle belongs to another pool"),
            Self::SlotOutOfRange { slot } => write!(formatter, "Drive slot {slot} is out of range"),
            Self::StaleHandle { slot } => write!(formatter, "Drive slot {slot} is stale"),
            Self::DriveIsFree { slot } => write!(formatter, "Drive slot {slot} is free"),
            Self::BodyPoolMismatch => {
                formatter.write_str("Drive pool is bound to a different body pool")
            }
            Self::BodyAccess {
                drive,
                endpoint,
                source,
            } => match drive {
                Some(drive) => write!(
                    formatter,
                    "Drive slot {drive} has invalid {endpoint:?} body access: {source}"
                ),
                None => write!(formatter, "invalid {endpoint:?} body handle: {source}"),
            },
            Self::FramesPoisoned { slot } => {
                write!(formatter, "Drive slot {slot} frames lock is poisoned")
            }
            Self::DynamicsPoisoned { slot } => {
                write!(formatter, "Drive slot {slot} dynamics lock is poisoned")
            }
            Self::EndpointMismatch { slot, endpoint } => {
                write!(
                    formatter,
                    "Drive slot {slot} has a different {endpoint:?} endpoint"
                )
            }
            Self::FramesOwnerMismatch { slot } => {
                write!(formatter, "Drive slot {slot} retains another frames owner")
            }
            Self::DynamicsOwnerMismatch { slot } => write!(
                formatter,
                "Drive slot {slot} retains another dynamics owner"
            ),
            Self::CorruptStructure => formatter.write_str("Drive pool structure is corrupt"),
        }
    }
}

impl Error for SimulationDrivePoolError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::BodyAccess { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Preallocated semantic Drive pool with Free and Active circular lists.
#[derive(Debug)]
pub struct SimulationDrivePool {
    authority: SimulationDrivePoolAuthority,
    body_pool_authority: SimulationRigidBodyPoolAuthority,
    slots: Box<[DriveSlot]>,
    anchors: [Links; 2],
    counts: SimulationDriveCounts,
    next_occurrence: Option<NonZeroU64>,
    /// Set by a successful [`Self::validate_integrity`]; every ring edit clears it.
    validated_topology: OnceLock<()>,
}

impl SimulationDrivePool {
    /// Constructs the two rings and inserts all slots into Free in ascending order.
    ///
    /// Native allocation failure is not recoverable inside `Simulation::Initialize`.
    /// The host reports it before publishing a partial pool.
    ///
    /// # Errors
    ///
    /// Returns an error if the native-width capacity cannot be represented,
    /// allocation fails, or process-unique authority is exhausted.
    pub fn try_new(
        capacity: usize,
        body_pool: &SimulationRigidBodyPool,
    ) -> Result<Self, SimulationDrivePoolError> {
        let capacity_u32 = u32::try_from(capacity)
            .map_err(|_| SimulationDrivePoolError::CapacityExceedsU32 { capacity })?;
        let mut storage = Vec::new();
        storage
            .try_reserve_exact(capacity)
            .map_err(|_| SimulationDrivePoolError::AllocationFailed { capacity })?;
        let authority = fresh_drive_pool_authority(&NEXT_DRIVE_POOL_AUTHORITY)?;

        for index in 0..capacity_u32 {
            let id = SimulationDriveId(index);
            let left = if index == 0 {
                SimulationDriveNode::Anchor(SimulationDriveList::Free)
            } else {
                SimulationDriveNode::Drive(SimulationDriveId(index - 1))
            };
            let right = if index + 1 == capacity_u32 {
                SimulationDriveNode::Anchor(SimulationDriveList::Free)
            } else {
                SimulationDriveNode::Drive(SimulationDriveId(index + 1))
            };
            storage.push(DriveSlot::initialized(id, Links { right, left }));
        }

        let mut anchors = SimulationDriveList::ALL.map(|list| {
            let node = SimulationDriveNode::Anchor(list);
            Links {
                right: node,
                left: node,
            }
        });
        if capacity_u32 != 0 {
            anchors[SimulationDriveList::Free.index()] = Links {
                right: SimulationDriveNode::Drive(SimulationDriveId(0)),
                left: SimulationDriveNode::Drive(SimulationDriveId(capacity_u32 - 1)),
            };
        }

        Ok(Self {
            authority,
            body_pool_authority: body_pool.authority(),
            slots: storage.into_boxed_slice(),
            anchors,
            counts: SimulationDriveCounts {
                free: capacity_u32,
                active: 0,
            },
            next_occurrence: NonZeroU64::new(1),
            validated_topology: OnceLock::new(),
        })
    }

    #[must_use]
    pub const fn authority(&self) -> SimulationDrivePoolAuthority {
        self.authority
    }

    #[must_use]
    pub(crate) const fn body_pool_authority(&self) -> SimulationRigidBodyPoolAuthority {
        self.body_pool_authority
    }

    #[must_use]
    pub const fn counts(&self) -> SimulationDriveCounts {
        self.counts
    }

    /// Applies the exact `AddDrive` pool transition and raw endpoint mapping.
    ///
    /// `parent` is retained in native `m_bodyB`; `child` is retained in
    /// native `m_bodyA`. Frames and dynamics are retained, not copied into the
    /// Drive slot. Add resets spy to zero and deliberately leaves tag untouched.
    /// `Ok(None)` is the native capacity failure and has no effects.
    ///
    /// Native `AddDrive` accepts raw pointers without validation. This public
    /// safe-host seam instead preflights that parent and child are live in the
    /// body pool bound at construction. The private transition after that
    /// preflight retains native raw-acceptance behavior and performs no body
    /// access. A full Drive pool still returns `Ok(None)` before body validation,
    /// matching the native zero-free short circuit.
    ///
    /// # Errors
    ///
    /// Returns an error without Drive-pool effects if body authority/liveness,
    /// host occurrence space, or retained-ring validation fails.
    pub fn try_add_drive(
        &mut self,
        body_pool: &SimulationRigidBodyPool,
        parent: SimulationRigidBodyHandle,
        child: SimulationRigidBodyHandle,
        frames: &Arc<RwLock<SimulationDriveFrames>>,
        dynamics: &Arc<RwLock<SimulationDriveDynamics>>,
    ) -> Result<Option<SimulationDriveHandle>, SimulationDrivePoolError> {
        if self.counts.free == 0 {
            return Ok(None);
        }
        self.validate_body_pool(body_pool)?;
        body_pool
            .try_observe(parent)
            .map_err(|source| SimulationDrivePoolError::BodyAccess {
                drive: None,
                endpoint: SimulationDriveEndpoint::Parent,
                source,
            })?;
        body_pool
            .try_observe(child)
            .map_err(|source| SimulationDrivePoolError::BodyAccess {
                drive: None,
                endpoint: SimulationDriveEndpoint::Child,
                source,
            })?;
        self.add_drive_prevalidated(parent, child, frames, dynamics)
    }

    /// Native-shaped transition after the public safe-host body preflight.
    /// This method intentionally does not inspect either opaque body handle.
    fn add_drive_prevalidated(
        &mut self,
        parent: SimulationRigidBodyHandle,
        child: SimulationRigidBodyHandle,
        frames: &Arc<RwLock<SimulationDriveFrames>>,
        dynamics: &Arc<RwLock<SimulationDriveDynamics>>,
    ) -> Result<Option<SimulationDriveHandle>, SimulationDrivePoolError> {
        self.validate_integrity()?;
        let occurrence = self
            .next_occurrence
            .ok_or(SimulationDrivePoolError::OccurrenceExhausted)?;
        let next_occurrence = occurrence.get().checked_add(1).and_then(NonZeroU64::new);
        let free_anchor = SimulationDriveNode::Anchor(SimulationDriveList::Free);
        let node = self.links(free_anchor)?.right;
        let SimulationDriveNode::Drive(id) = node else {
            return Err(SimulationDrivePoolError::CorruptStructure);
        };
        let retained_frames = Arc::clone(frames);
        let retained_dynamics = Arc::clone(dynamics);

        self.unlink(node);
        self.append_to_tail(node, SimulationDriveList::Active);
        {
            let slot = &mut self.slots[id.0 as usize];
            slot.spy = Some(0);
            slot.frames = Some(retained_frames);
            slot.dynamics = Some(retained_dynamics);
            slot.body_b = Some(parent);
            slot.body_a = Some(child);
            // Native m_tag is intentionally untouched.
            slot.occurrence = Some(occurrence);
        }
        self.counts.free = self.counts.free.wrapping_sub(1);
        self.counts.active = self.counts.active.wrapping_add(1);
        self.next_occurrence = next_occurrence;

        Ok(Some(SimulationDriveHandle {
            authority: self.authority,
            id,
            occurrence,
        }))
    }

    /// Returns a Drive to the Free tail without reading either stored body.
    ///
    /// This is intentionally body-independent: native Assembly teardown removes
    /// bodies before Drives. All payload, including frames/dynamics owners,
    /// endpoints, tag, spy, and occurrence, remains installed after removal.
    /// No callback or implicit Drop removal exists.
    ///
    /// # Errors
    ///
    /// Returns a typed error with zero pool effects for a foreign, stale,
    /// out-of-range, already-free, or structurally invalid handle.
    pub fn try_remove_drive(
        &mut self,
        handle: SimulationDriveHandle,
    ) -> Result<(), SimulationDrivePoolError> {
        let index = self.validate_handle(handle)?;
        self.validate_integrity()?;
        self.remove_validated_slot(index);
        Ok(())
    }

    /// Actual `BoneDrives::CreateDrive` spy store (0x82b48d04), preserving tag,
    /// endpoints, payload owners and rings. Host identity checks precede writes.
    pub(crate) fn try_set_spy(
        &mut self,
        handle: SimulationDriveHandle,
        spy: u32,
    ) -> Result<(), SimulationDrivePoolError> {
        let index = self.validate_handle(handle)?;
        self.validate_integrity()?;
        self.slots[index].spy = Some(spy);
        Ok(())
    }

    /// Copies the current frames value through an occurrence-bound read identity.
    ///
    /// The shared owner and its write capability remain private. The caller that
    /// originally supplied the owner remains the explicit live-mutation path.
    /// Poison is reported rather than converted into a cached/default payload.
    ///
    /// # Errors
    ///
    /// Returns a typed identity/structural error, `FramesPoisoned`, or
    /// `CorruptStructure` if an active slot has no retained frames.
    pub fn try_frames(
        &self,
        identity: SimulationDriveIdentity,
    ) -> Result<SimulationDriveFrames, SimulationDrivePoolError> {
        let index = self.validate_identity(identity)?;
        self.validate_integrity()?;
        let owner = self.slots[index]
            .frames
            .as_ref()
            .ok_or(SimulationDrivePoolError::CorruptStructure)?;
        owner
            .read()
            .map(|frames| *frames)
            .map_err(|_| SimulationDrivePoolError::FramesPoisoned {
                slot: identity.id.0,
            })
    }

    /// Read-only host owner preflight before mutating the original live payload.
    /// Unlike Build, this never locks or copies numeric payloads. Exact `Arc`
    /// identity matters: equal values in replacement storage are not the native
    /// retained pointer. A recorded capacity null remains false, not a made-up
    /// active Drive, while both original endpoint occurrences are still checked.
    pub(crate) fn try_validate_owned_payloads(
        &self,
        body_pool: &SimulationRigidBodyPool,
        drive: Option<SimulationDriveHandle>,
        parent: SimulationRigidBodyHandle,
        child: SimulationRigidBodyHandle,
        frames: &Arc<RwLock<SimulationDriveFrames>>,
        dynamics: &Arc<RwLock<SimulationDriveDynamics>>,
    ) -> Result<bool, SimulationDrivePoolError> {
        self.validate_body_pool(body_pool)?;
        self.validate_integrity()?;
        let index = drive
            .map(|handle| self.validate_handle(handle))
            .transpose()?;
        for (body, endpoint) in [
            (parent, SimulationDriveEndpoint::Parent),
            (child, SimulationDriveEndpoint::Child),
        ] {
            body_pool
                .try_validate_observation(body.observation())
                .map_err(|source| SimulationDrivePoolError::BodyAccess {
                    drive: drive.map(|handle| handle.id().index()),
                    endpoint,
                    source,
                })?;
        }
        let Some(index) = index else {
            return Ok(false);
        };
        let slot = &self.slots[index];
        for (retained, expected, endpoint) in [
            (slot.body_b, parent, SimulationDriveEndpoint::Parent),
            (slot.body_a, child, SimulationDriveEndpoint::Child),
        ] {
            let retained = retained.ok_or(SimulationDrivePoolError::CorruptStructure)?;
            if retained != expected {
                return Err(SimulationDrivePoolError::EndpointMismatch {
                    slot: slot.id.0,
                    endpoint,
                });
            }
        }
        let retained_frames = slot
            .frames
            .as_ref()
            .ok_or(SimulationDrivePoolError::CorruptStructure)?;
        if !Arc::ptr_eq(retained_frames, frames) {
            return Err(SimulationDrivePoolError::FramesOwnerMismatch { slot: slot.id.0 });
        }
        let retained_dynamics = slot
            .dynamics
            .as_ref()
            .ok_or(SimulationDrivePoolError::CorruptStructure)?;
        if !Arc::ptr_eq(retained_dynamics, dynamics) {
            return Err(SimulationDrivePoolError::DynamicsOwnerMismatch { slot: slot.id.0 });
        }
        Ok(true)
    }

    /// Copies the current dynamics value through an occurrence-bound read identity.
    ///
    /// The shared owner and its write capability remain private. The caller that
    /// originally supplied the owner remains the explicit live-mutation path.
    /// Poison is reported rather than converted into a cached/default payload.
    ///
    /// # Errors
    ///
    /// Returns a typed identity/structural error, `DynamicsPoisoned`, or
    /// `CorruptStructure` if an active slot has no retained dynamics.
    pub fn try_dynamics(
        &self,
        identity: SimulationDriveIdentity,
    ) -> Result<SimulationDriveDynamics, SimulationDrivePoolError> {
        let index = self.validate_identity(identity)?;
        self.validate_integrity()?;
        let owner = self.slots[index]
            .dynamics
            .as_ref()
            .ok_or(SimulationDrivePoolError::CorruptStructure)?;
        owner.read().map(|dynamics| *dynamics).map_err(|_| {
            SimulationDrivePoolError::DynamicsPoisoned {
                slot: identity.id.0,
            }
        })
    }

    /// Reproduces `DriveBatchBuild`'s active-list order and either-body-active gate.
    ///
    /// Native `0x82ae7a14..0x82ae7a30` loads semantic parent/bodyB state first,
    /// then child/bodyA state, and admits the Drive when either has bit 4. The
    /// safe host validates occurrence-bound read observations against the bound
    /// body pool in that same order. The result contains occurrence-bound inert
    /// Drive identities, not removal handles. This method builds no Jacobian and
    /// mutates no pool state; solver math remains outside this module.
    ///
    /// # Errors
    ///
    /// Returns a structural/body-pool error or identifies the first stale,
    /// foreign, or free endpoint in native parent-then-child access order.
    pub fn try_active_build_order(
        &self,
        body_pool: &SimulationRigidBodyPool,
    ) -> Result<Vec<SimulationDriveIdentity>, SimulationDrivePoolError> {
        self.validate_body_pool(body_pool)?;
        self.validate_integrity()?;
        let mut result = Vec::new();
        let anchor = SimulationDriveNode::Anchor(SimulationDriveList::Active);
        let mut node = self.anchors[SimulationDriveList::Active.index()].right;
        while node != anchor {
            let SimulationDriveNode::Drive(id) = node else {
                return Err(SimulationDrivePoolError::CorruptStructure);
            };
            let slot = &self.slots[id.0 as usize];
            let parent = slot
                .body_b
                .ok_or(SimulationDrivePoolError::CorruptStructure)?;
            let child = slot
                .body_a
                .ok_or(SimulationDrivePoolError::CorruptStructure)?;
            let parent_state = body_pool
                .try_raw_state(parent.observation())
                .map_err(|source| SimulationDrivePoolError::BodyAccess {
                    drive: Some(id.0),
                    endpoint: SimulationDriveEndpoint::Parent,
                    source,
                })?;
            let child_state = body_pool
                .try_raw_state(child.observation())
                .map_err(|source| SimulationDrivePoolError::BodyAccess {
                    drive: Some(id.0),
                    endpoint: SimulationDriveEndpoint::Child,
                    source,
                })?;
            if (parent_state | child_state) & ACTIVE_BODY_BIT != 0 {
                let occurrence = slot
                    .occurrence
                    .ok_or(SimulationDrivePoolError::CorruptStructure)?;
                result.push(SimulationDriveIdentity {
                    authority: self.authority,
                    id,
                    occurrence,
                });
            }
            node = slot.links.right;
        }
        Ok(result)
    }

    /// Copies every admitted active Drive input in native right-ring order.
    ///
    /// Admission preserves `DriveBatchBuild`'s unconditional parent/body-B
    /// state read followed by child/body-A state read and its exact
    /// `((state_b | state_a) & 4) != 0` predicate. Rejected Drives do not lock
    /// or copy frames/dynamics. For admitted Drives, this safe-host boundary
    /// then detaches all Build values and retains occurrence-bound body
    /// observations for a future solver sidecar.
    ///
    /// Native trusts its rings, writes `DS_Count=0`, publishes each incremented
    /// `DR_Count` before calling void Build, and cannot roll back. This helper
    /// deliberately has no native-count side effects: it validates and reserves
    /// the complete temporary result first, and an error exposes no partial
    /// batch. Successful ordering and admission remain native-equivalent.
    ///
    /// # Errors
    ///
    /// Returns a typed structural, allocation, body-occurrence, or poisoned-lock
    /// error without mutating either pool or publishing any partial result.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "reserved for the private Drive Jacobian batch seam"
        )
    )]
    #[allow(
        clippy::similar_names,
        reason = "body A/child and body B/parent roles must remain explicit"
    )]
    pub(crate) fn try_active_build_snapshots(
        &self,
        body_pool: &SimulationRigidBodyPool,
    ) -> Result<Vec<SimulationDriveBuildSnapshot>, SimulationDrivePoolError> {
        self.validate_body_pool(body_pool)?;
        self.validate_integrity()?;

        let capacity = self.counts.active as usize;
        let mut result = Vec::new();
        result
            .try_reserve_exact(capacity)
            .map_err(|_| SimulationDrivePoolError::AllocationFailed { capacity })?;

        let anchor = SimulationDriveNode::Anchor(SimulationDriveList::Active);
        let mut node = self.anchors[SimulationDriveList::Active.index()].right;
        while node != anchor {
            let SimulationDriveNode::Drive(id) = node else {
                return Err(SimulationDrivePoolError::CorruptStructure);
            };
            let slot = &self.slots[id.0 as usize];
            let body_b = slot
                .body_b
                .ok_or(SimulationDrivePoolError::CorruptStructure)?;
            let body_a = slot
                .body_a
                .ok_or(SimulationDrivePoolError::CorruptStructure)?;
            let body_b_observation = body_b.observation();
            let body_a_observation = body_a.observation();

            let state_b = body_pool
                .try_raw_state(body_b_observation)
                .map_err(|source| SimulationDrivePoolError::BodyAccess {
                    drive: Some(id.0),
                    endpoint: SimulationDriveEndpoint::Parent,
                    source,
                })?;
            let state_a = body_pool
                .try_raw_state(body_a_observation)
                .map_err(|source| SimulationDrivePoolError::BodyAccess {
                    drive: Some(id.0),
                    endpoint: SimulationDriveEndpoint::Child,
                    source,
                })?;

            if (state_b | state_a) & ACTIVE_BODY_BIT != 0 {
                let body_a_snapshot = body_pool
                    .try_drive_build_snapshot(body_a_observation)
                    .map_err(|source| SimulationDrivePoolError::BodyAccess {
                        drive: Some(id.0),
                        endpoint: SimulationDriveEndpoint::Child,
                        source,
                    })?;
                let body_b_snapshot = body_pool
                    .try_drive_build_snapshot(body_b_observation)
                    .map_err(|source| SimulationDrivePoolError::BodyAccess {
                        drive: Some(id.0),
                        endpoint: SimulationDriveEndpoint::Parent,
                        source,
                    })?;
                let frames_owner = slot
                    .frames
                    .as_ref()
                    .ok_or(SimulationDrivePoolError::CorruptStructure)?;
                let frames = frames_owner
                    .read()
                    .map(|frames| *frames)
                    .map_err(|_| SimulationDrivePoolError::FramesPoisoned { slot: id.0 })?;
                let dynamics_owner = slot
                    .dynamics
                    .as_ref()
                    .ok_or(SimulationDrivePoolError::CorruptStructure)?;
                let dynamics = dynamics_owner
                    .read()
                    .map(|dynamics| *dynamics)
                    .map_err(|_| SimulationDrivePoolError::DynamicsPoisoned { slot: id.0 })?;
                let occurrence = slot
                    .occurrence
                    .ok_or(SimulationDrivePoolError::CorruptStructure)?;
                let spy = slot.spy.ok_or(SimulationDrivePoolError::CorruptStructure)?;
                result.push(SimulationDriveBuildSnapshot {
                    drive: SimulationDriveIdentity {
                        authority: self.authority,
                        id,
                        occurrence,
                    },
                    body_a: body_a_snapshot,
                    body_b: body_b_snapshot,
                    frames,
                    dynamics,
                    time_step: body_pool.timestep(),
                    spy,
                });
            }
            node = slot.links.right;
        }
        Ok(result)
    }

    /// Copies every admitted active Drive input through one live `DynamicUpdate`
    /// phase in native right-ring order.
    ///
    /// This is the executable preparation path. The phase keeps body topology,
    /// body numeric fields, and the frame timestep stable across the complete
    /// B-state, A-state, A-snapshot, B-snapshot traversal. Drive frames and
    /// dynamics retain their native build-time snapshot behavior.
    #[allow(
        clippy::similar_names,
        reason = "body A/child and body B/parent roles must remain explicit"
    )]
    pub(crate) fn try_active_build_snapshots_for_phase(
        &self,
        phase: &SimulationRigidBodyDynamicUpdatePhase<'_>,
    ) -> Result<Vec<SimulationDriveBuildSnapshot>, SimulationDrivePoolError> {
        if phase.body_pool_authority() != self.body_pool_authority {
            return Err(SimulationDrivePoolError::BodyPoolMismatch);
        }
        self.validate_integrity()?;

        let capacity = self.counts.active as usize;
        let mut result = Vec::new();
        result
            .try_reserve_exact(capacity)
            .map_err(|_| SimulationDrivePoolError::AllocationFailed { capacity })?;

        let anchor = SimulationDriveNode::Anchor(SimulationDriveList::Active);
        let mut node = self.anchors[SimulationDriveList::Active.index()].right;
        while node != anchor {
            let SimulationDriveNode::Drive(id) = node else {
                return Err(SimulationDrivePoolError::CorruptStructure);
            };
            let slot = &self.slots[id.0 as usize];
            let body_b = slot
                .body_b
                .ok_or(SimulationDrivePoolError::CorruptStructure)?;
            let body_a = slot
                .body_a
                .ok_or(SimulationDrivePoolError::CorruptStructure)?;
            let body_b_observation = body_b.observation();
            let body_a_observation = body_a.observation();

            let state_b = phase
                .try_raw_state_for_constraint(body_b_observation)
                .map_err(|source| SimulationDrivePoolError::BodyAccess {
                    drive: Some(id.0),
                    endpoint: SimulationDriveEndpoint::Parent,
                    source,
                })?;
            let state_a = phase
                .try_raw_state_for_constraint(body_a_observation)
                .map_err(|source| SimulationDrivePoolError::BodyAccess {
                    drive: Some(id.0),
                    endpoint: SimulationDriveEndpoint::Child,
                    source,
                })?;

            if (state_b | state_a) & ACTIVE_BODY_BIT != 0 {
                let body_a_snapshot =
                    phase
                        .try_drive_build_snapshot(body_a_observation)
                        .map_err(|source| SimulationDrivePoolError::BodyAccess {
                            drive: Some(id.0),
                            endpoint: SimulationDriveEndpoint::Child,
                            source,
                        })?;
                let body_b_snapshot =
                    phase
                        .try_drive_build_snapshot(body_b_observation)
                        .map_err(|source| SimulationDrivePoolError::BodyAccess {
                            drive: Some(id.0),
                            endpoint: SimulationDriveEndpoint::Parent,
                            source,
                        })?;
                let frames_owner = slot
                    .frames
                    .as_ref()
                    .ok_or(SimulationDrivePoolError::CorruptStructure)?;
                let frames = frames_owner
                    .read()
                    .map(|frames| *frames)
                    .map_err(|_| SimulationDrivePoolError::FramesPoisoned { slot: id.0 })?;
                let dynamics_owner = slot
                    .dynamics
                    .as_ref()
                    .ok_or(SimulationDrivePoolError::CorruptStructure)?;
                let dynamics = dynamics_owner
                    .read()
                    .map(|dynamics| *dynamics)
                    .map_err(|_| SimulationDrivePoolError::DynamicsPoisoned { slot: id.0 })?;
                let occurrence = slot
                    .occurrence
                    .ok_or(SimulationDrivePoolError::CorruptStructure)?;
                let spy = slot.spy.ok_or(SimulationDrivePoolError::CorruptStructure)?;
                result.push(SimulationDriveBuildSnapshot {
                    drive: SimulationDriveIdentity {
                        authority: self.authority,
                        id,
                        occurrence,
                    },
                    body_a: body_a_snapshot,
                    body_b: body_b_snapshot,
                    frames,
                    dynamics,
                    time_step: phase.timestep(),
                    spy,
                });
            }
            node = slot.links.right;
        }
        Ok(result)
    }

    /// Revalidates the occurrence and endpoint sidecars of a prepared batch.
    ///
    /// The complete Drive structure is checked once, then bindings are checked
    /// in supplied order without acquiring frames/dynamics locks or reading RF.
    /// Occurrence validation is host ABA protection; it is not a native epoch.
    ///
    /// # Errors
    ///
    /// Returns before any mutation for a foreign body pool, invalid Drive
    /// occurrence, changed semantic endpoint, or corrupt retained structure.
    #[cfg(test)]
    pub(crate) fn try_validate_build_bindings(
        &self,
        body_pool: &SimulationRigidBodyPool,
        bindings: impl IntoIterator<Item = SimulationDriveBuildBinding>,
    ) -> Result<(), SimulationDriveBuildBindingValidationError> {
        self.validate_body_pool(body_pool)
            .map_err(SimulationDriveBuildBindingValidationError::Pool)?;
        self.try_validate_build_bindings_for_body_pool_authority(body_pool.authority(), bindings)
    }

    /// Revalidates Drive occurrences/endpoints against an inert body authority.
    ///
    /// Body occurrence liveness is intentionally left to the exclusive RF
    /// phase that immediately consumes the same ordered observations.
    pub(crate) fn try_validate_build_bindings_for_body_pool_authority(
        &self,
        body_pool: SimulationRigidBodyPoolAuthority,
        bindings: impl IntoIterator<Item = SimulationDriveBuildBinding>,
    ) -> Result<(), SimulationDriveBuildBindingValidationError> {
        if self.body_pool_authority != body_pool {
            return Err(SimulationDriveBuildBindingValidationError::Pool(
                SimulationDrivePoolError::BodyPoolMismatch,
            ));
        }
        self.validate_integrity()
            .map_err(SimulationDriveBuildBindingValidationError::Pool)?;
        for (ordinal, binding) in bindings.into_iter().enumerate() {
            let map_binding_error = |source| SimulationDriveBuildBindingValidationError::Binding {
                ordinal,
                drive: binding.drive,
                source,
            };
            let index = self
                .validate_identity(binding.drive)
                .map_err(map_binding_error)?;
            let slot = &self.slots[index];
            let body_a = slot
                .body_a
                .ok_or(SimulationDrivePoolError::CorruptStructure)
                .map_err(map_binding_error)?;
            if body_a.observation() != binding.body_a {
                return Err(
                    SimulationDriveBuildBindingValidationError::EndpointMismatch {
                        ordinal,
                        drive: binding.drive,
                        endpoint: SimulationDriveEndpoint::Child,
                    },
                );
            }
            let body_b = slot
                .body_b
                .ok_or(SimulationDrivePoolError::CorruptStructure)
                .map_err(map_binding_error)?;
            if body_b.observation() != binding.body_b {
                return Err(
                    SimulationDriveBuildBindingValidationError::EndpointMismatch {
                        ordinal,
                        drive: binding.drive,
                        endpoint: SimulationDriveEndpoint::Parent,
                    },
                );
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn snapshot(&self) -> SimulationDrivePoolSnapshot {
        SimulationDrivePoolSnapshot {
            authority: self.authority,
            body_pool_authority: self.body_pool_authority,
            counts: self.counts,
            next_occurrence: self.next_occurrence.map(NonZeroU64::get),
            free_order: self.list_order(SimulationDriveList::Free),
            active_order: self.list_order(SimulationDriveList::Active),
            slots: self
                .slots
                .iter()
                .map(|slot| SimulationDriveSlotSnapshot {
                    id: slot.id,
                    occurrence: slot.occurrence.map(NonZeroU64::get),
                    frames_present: slot.frames.is_some(),
                    dynamics_present: slot.dynamics.is_some(),
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
        handle: SimulationDriveHandle,
    ) -> Result<usize, SimulationDrivePoolError> {
        if handle.authority != self.authority {
            return Err(SimulationDrivePoolError::ForeignHandle);
        }
        let index = handle.id.0 as usize;
        let slot = self
            .slots
            .get(index)
            .ok_or(SimulationDrivePoolError::SlotOutOfRange { slot: handle.id.0 })?;
        if slot.occurrence != Some(handle.occurrence) {
            return Err(SimulationDrivePoolError::StaleHandle { slot: handle.id.0 });
        }
        if slot.membership == SimulationDriveList::Free {
            return Err(SimulationDrivePoolError::DriveIsFree { slot: handle.id.0 });
        }
        Ok(index)
    }

    fn validate_identity(
        &self,
        identity: SimulationDriveIdentity,
    ) -> Result<usize, SimulationDrivePoolError> {
        self.validate_handle(SimulationDriveHandle {
            authority: identity.authority,
            id: identity.id,
            occurrence: identity.occurrence,
        })
    }

    pub(crate) fn validate_body_pool(
        &self,
        body_pool: &SimulationRigidBodyPool,
    ) -> Result<(), SimulationDrivePoolError> {
        if body_pool.authority() != self.body_pool_authority {
            return Err(SimulationDrivePoolError::BodyPoolMismatch);
        }
        Ok(())
    }

    fn remove_validated_slot(&mut self, index: usize) {
        let node = SimulationDriveNode::Drive(self.slots[index].id);
        self.unlink(node);
        self.append_to_tail(node, SimulationDriveList::Free);
        self.counts.active = self.counts.active.wrapping_sub(1);
        self.counts.free = self.counts.free.wrapping_add(1);
    }

    fn unlink(&mut self, node: SimulationDriveNode) {
        let links = self.links(node).expect("validated Drive node has links");
        self.links_mut(links.right)
            .expect("validated right neighbor")
            .left = links.left;
        self.links_mut(links.left)
            .expect("validated left neighbor")
            .right = links.right;
    }

    fn append_to_tail(&mut self, node: SimulationDriveNode, list: SimulationDriveList) {
        let anchor = SimulationDriveNode::Anchor(list);
        let tail = self.links(anchor).expect("Drive anchor exists").left;
        {
            let links = self.links_mut(node).expect("validated Drive node");
            links.right = anchor;
            links.left = tail;
        }
        self.links_mut(tail).expect("validated Drive tail").right = node;
        self.links_mut(anchor).expect("Drive anchor exists").left = node;
        if let SimulationDriveNode::Drive(id) = node {
            self.slots[id.0 as usize].membership = list;
        }
    }

    fn links(&self, node: SimulationDriveNode) -> Result<Links, SimulationDrivePoolError> {
        match node {
            SimulationDriveNode::Anchor(list) => Ok(self.anchors[list.index()]),
            SimulationDriveNode::Drive(id) => self
                .slots
                .get(id.0 as usize)
                .map(|slot| slot.links)
                .ok_or(SimulationDrivePoolError::CorruptStructure),
        }
    }

    fn links_mut(&mut self, node: SimulationDriveNode) -> Option<&mut Links> {
        // All Add/Remove ring edits pass here. Membership, count, endpoint,
        // payload and occurrence stores in those operations complete before
        // the next validation; spy stores keep an Active slot's spy present.
        self.validated_topology.take();
        match node {
            SimulationDriveNode::Anchor(list) => self.anchors.get_mut(list.index()),
            SimulationDriveNode::Drive(id) => self
                .slots
                .get_mut(id.0 as usize)
                .map(|slot| &mut slot.links),
        }
    }

    fn validate_integrity(&self) -> Result<(), SimulationDrivePoolError> {
        if self.validated_topology.get().is_some() {
            return Ok(());
        }
        let mut seen = vec![false; self.slots.len()];
        for list in SimulationDriveList::ALL {
            let anchor = SimulationDriveNode::Anchor(list);
            let anchor_links = self.links(anchor)?;
            if self.links(anchor_links.right)?.left != anchor
                || self.links(anchor_links.left)?.right != anchor
            {
                return Err(SimulationDrivePoolError::CorruptStructure);
            }
            let mut node = anchor_links.right;
            let mut count = 0_u32;
            while node != anchor {
                let SimulationDriveNode::Drive(id) = node else {
                    return Err(SimulationDrivePoolError::CorruptStructure);
                };
                let index = id.0 as usize;
                let Some(slot) = self.slots.get(index) else {
                    return Err(SimulationDrivePoolError::CorruptStructure);
                };
                if seen[index] || slot.id != id || slot.membership != list {
                    return Err(SimulationDrivePoolError::CorruptStructure);
                }
                if list == SimulationDriveList::Active
                    && (slot.occurrence.is_none()
                        || slot.frames.is_none()
                        || slot.dynamics.is_none()
                        || slot.body_a.is_none()
                        || slot.body_b.is_none()
                        || slot.spy.is_none())
                {
                    return Err(SimulationDrivePoolError::CorruptStructure);
                }
                let links = slot.links;
                if self.links(links.right)?.left != node || self.links(links.left)?.right != node {
                    return Err(SimulationDrivePoolError::CorruptStructure);
                }
                seen[index] = true;
                count = count.wrapping_add(1);
                if count as usize > self.slots.len() {
                    return Err(SimulationDrivePoolError::CorruptStructure);
                }
                node = links.right;
            }
            if count != self.counts.get(list) {
                return Err(SimulationDrivePoolError::CorruptStructure);
            }
        }
        if seen.into_iter().any(|present| !present) {
            return Err(SimulationDrivePoolError::CorruptStructure);
        }
        let _ = self.validated_topology.set(());
        Ok(())
    }

    fn list_order(&self, list: SimulationDriveList) -> Vec<SimulationDriveId> {
        let anchor = SimulationDriveNode::Anchor(list);
        let mut node = self.anchors[list.index()].right;
        let mut result = Vec::new();
        while node != anchor && result.len() < self.slots.len() {
            let SimulationDriveNode::Drive(id) = node else {
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

fn fresh_drive_pool_authority(
    source: &AtomicU64,
) -> Result<SimulationDrivePoolAuthority, SimulationDrivePoolError> {
    source
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .map(SimulationDrivePoolAuthority)
        .map_err(|_| SimulationDrivePoolError::AuthorityExhausted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation_rigid_body_pool::SimulationRigidBodyPool;

    fn identity() -> [f32; 16] {
        [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]
    }

    fn frames(seed: u32) -> Arc<RwLock<SimulationDriveFrames>> {
        let value = |offset| f32::from_bits(seed.wrapping_add(offset));
        Arc::new(RwLock::new(SimulationDriveFrames {
            quaternion_a: [value(0), value(1), value(2), value(3)],
            position_a: [value(4), value(5), value(6), value(7)],
            quaternion_b: [value(8), value(9), value(10), value(11)],
            position_b: [value(12), value(13), value(14), value(15)],
        }))
    }

    fn dynamics(seed: u32) -> Arc<RwLock<SimulationDriveDynamics>> {
        let value = |offset| f32::from_bits(seed.wrapping_add(offset));
        Arc::new(RwLock::new(SimulationDriveDynamics {
            linear: SimulationDriveDynamicsParams {
                spring: value(0),
                damping: value(1),
                strength: value(2),
                raw_type: 0x8000_0001,
            },
            angular: SimulationDriveDynamicsParams {
                spring: value(4),
                damping: value(5),
                strength: value(6),
                raw_type: 2,
            },
        }))
    }

    fn frame_bits(value: SimulationDriveFrames) -> [u32; 16] {
        let mut bits = [0; 16];
        for (destination, component) in bits.iter_mut().zip(
            [
                value.quaternion_a,
                value.position_a,
                value.quaternion_b,
                value.position_b,
            ]
            .into_iter()
            .flatten(),
        ) {
            *destination = component.to_bits();
        }
        bits
    }

    fn dynamics_bits(value: SimulationDriveDynamics) -> [u32; 8] {
        [
            value.linear.spring.to_bits(),
            value.linear.damping.to_bits(),
            value.linear.strength.to_bits(),
            value.linear.raw_type,
            value.angular.spring.to_bits(),
            value.angular.damping.to_bits(),
            value.angular.strength.to_bits(),
            value.angular.raw_type,
        ]
    }

    fn bodies(raw_states: &[u32]) -> (SimulationRigidBodyPool, Vec<SimulationRigidBodyHandle>) {
        let mut pool = SimulationRigidBodyPool::try_new(raw_states.len()).expect("body pool");
        let handles = raw_states
            .iter()
            .map(|state| {
                pool.try_add_rigid_body(identity(), None, *state)
                    .expect("body pool")
                    .expect("body capacity")
            })
            .collect();
        (pool, handles)
    }

    fn add(
        pool: &mut SimulationDrivePool,
        body_pool: &SimulationRigidBodyPool,
        parent: SimulationRigidBodyHandle,
        child: SimulationRigidBodyHandle,
        frames: &Arc<RwLock<SimulationDriveFrames>>,
        dynamics: &Arc<RwLock<SimulationDriveDynamics>>,
    ) -> SimulationDriveHandle {
        pool.try_add_drive(body_pool, parent, child, frames, dynamics)
            .expect("Drive pool")
            .expect("Drive capacity")
    }

    #[test]
    fn initialization_builds_two_rings_and_leaves_payload_unknown() {
        let body_pool = SimulationRigidBodyPool::try_new(0).expect("body pool");
        let empty = SimulationDrivePool::try_new(0, &body_pool).expect("pool");
        let snapshot = empty.snapshot();
        assert_eq!(snapshot.counts, SimulationDriveCounts::default());
        assert!(snapshot.free_order.is_empty());
        assert!(snapshot.active_order.is_empty());
        assert!(snapshot.slots.is_empty());

        let pool = SimulationDrivePool::try_new(3, &body_pool).expect("pool");
        let snapshot = pool.snapshot();
        assert_eq!(
            snapshot.counts,
            SimulationDriveCounts { free: 3, active: 0 }
        );
        assert_eq!(
            snapshot.free_order,
            [
                SimulationDriveId(0),
                SimulationDriveId(1),
                SimulationDriveId(2)
            ]
        );
        assert!(snapshot.active_order.is_empty());
        for (index, slot) in snapshot.slots.iter().enumerate() {
            assert_eq!(slot.id.index() as usize, index);
            assert_eq!(slot.occurrence, None);
            assert!(!slot.frames_present);
            assert!(!slot.dynamics_present);
            assert_eq!(slot.body_a, None);
            assert_eq!(slot.body_b, None);
            assert_eq!(slot.tag, None);
            assert_eq!(slot.spy, None);
            assert_eq!(slot.membership, SimulationDriveList::Free);
        }
    }

    #[test]
    fn retained_payload_layouts_match_pdb_sizes_and_vector_alignment() {
        assert_eq!(std::mem::size_of::<SimulationDriveFrames>(), 0x40);
        assert_eq!(std::mem::align_of::<SimulationDriveFrames>(), 0x10);
        assert_eq!(std::mem::size_of::<SimulationDriveDynamicsParams>(), 0x10);
        assert_eq!(std::mem::align_of::<SimulationDriveDynamicsParams>(), 4);
        assert_eq!(std::mem::size_of::<SimulationDriveDynamics>(), 0x20);
        assert_eq!(std::mem::align_of::<SimulationDriveDynamics>(), 4);
    }

    #[test]
    fn add_preserves_raw_parent_child_mapping_and_all_input_bits() {
        let (body_pool, body) = bodies(&[4, 1]);
        let frames = frames(0x7fc0_1000);
        let dynamics = dynamics(0x8000_2000);
        let mut pool = SimulationDrivePool::try_new(1, &body_pool).expect("pool");
        let handle = add(&mut pool, &body_pool, body[0], body[1], &frames, &dynamics);
        let snapshot = pool.snapshot();
        let slot = &snapshot.slots[0];
        assert_eq!(handle.id(), SimulationDriveId(0));
        assert_eq!(snapshot.free_order, []);
        assert_eq!(snapshot.active_order, [handle.id()]);
        assert_eq!(
            snapshot.counts,
            SimulationDriveCounts { free: 0, active: 1 }
        );
        assert_eq!(
            slot.body_b,
            Some(body[0].identity()),
            "native bodyB is parent"
        );
        assert_eq!(
            slot.body_a,
            Some(body[1].identity()),
            "native bodyA is child"
        );
        assert!(slot.frames_present);
        assert!(slot.dynamics_present);
        assert_eq!(
            frame_bits(*frames.read().expect("frames read")),
            [
                0x7fc0_1000,
                0x7fc0_1001,
                0x7fc0_1002,
                0x7fc0_1003,
                0x7fc0_1004,
                0x7fc0_1005,
                0x7fc0_1006,
                0x7fc0_1007,
                0x7fc0_1008,
                0x7fc0_1009,
                0x7fc0_100a,
                0x7fc0_100b,
                0x7fc0_100c,
                0x7fc0_100d,
                0x7fc0_100e,
                0x7fc0_100f,
            ]
        );
        assert_eq!(
            dynamics_bits(*dynamics.read().expect("dynamics read")),
            [
                0x8000_2000,
                0x8000_2001,
                0x8000_2002,
                0x8000_0001,
                0x8000_2004,
                0x8000_2005,
                0x8000_2006,
                2,
            ]
        );
        assert_eq!(slot.spy, Some(0));
        assert_eq!(slot.tag, None);
        assert_eq!(
            frame_bits(pool.try_frames(handle.identity()).expect("frames")),
            frame_bits(*frames.read().expect("source frames"))
        );
        assert_eq!(
            dynamics_bits(pool.try_dynamics(handle.identity()).expect("dynamics")),
            dynamics_bits(*dynamics.read().expect("source dynamics"))
        );
    }

    #[test]
    fn retained_live_inputs_observe_mutations_made_after_add() {
        let (body_pool, body) = bodies(&[4, 4]);
        let frames = frames(0x3f80_0000);
        let dynamics = dynamics(0x4000_0000);
        let mut pool = SimulationDrivePool::try_new(1, &body_pool).expect("pool");
        let drive = add(&mut pool, &body_pool, body[0], body[1], &frames, &dynamics);

        frames.write().expect("frames write").position_b[2] = f32::from_bits(0xffc0_4321);
        dynamics.write().expect("dynamics write").angular.strength = f32::from_bits(0x8000_0000);

        let retained_frames = pool.try_frames(drive.identity()).expect("retained frames");
        let retained_dynamics = pool
            .try_dynamics(drive.identity())
            .expect("retained dynamics");
        assert_eq!(retained_frames.position_b[2].to_bits(), 0xffc0_4321);
        assert_eq!(retained_dynamics.angular.strength.to_bits(), 0x8000_0000);
    }

    #[test]
    fn poisoned_input_locks_remain_visible_and_do_not_block_add_or_remove() {
        let (body_pool, body) = bodies(&[4, 4]);
        let frames = frames(0x3f80_0000);
        let dynamics = dynamics(0x4000_0000);
        let _ = std::panic::catch_unwind({
            let frames = Arc::clone(&frames);
            move || {
                let _guard = frames.write().expect("initial write");
                panic!("poison frames");
            }
        });
        let _ = std::panic::catch_unwind({
            let dynamics = Arc::clone(&dynamics);
            move || {
                let _guard = dynamics.write().expect("initial write");
                panic!("poison dynamics");
            }
        });

        let mut pool = SimulationDrivePool::try_new(1, &body_pool).expect("pool");
        let drive = add(&mut pool, &body_pool, body[0], body[1], &frames, &dynamics);
        assert_eq!(
            pool.try_frames(drive.identity())
                .expect_err("poisoned frames"),
            SimulationDrivePoolError::FramesPoisoned { slot: 0 }
        );
        assert_eq!(
            pool.try_dynamics(drive.identity())
                .expect_err("poisoned dynamics"),
            SimulationDrivePoolError::DynamicsPoisoned { slot: 0 }
        );
        assert!(pool.snapshot().slots[0].frames_present);
        assert!(pool.snapshot().slots[0].dynamics_present);
        pool.try_remove_drive(drive)
            .expect("remove performs no payload lock");
    }

    #[test]
    fn add_rejects_foreign_child_before_drive_effects() {
        let (parent_pool, parent) = bodies(&[4]);
        let (_child_pool, child) = bodies(&[4]);
        let frames = frames(0x3f80_0000);
        let dynamics = dynamics(0x4000_0000);
        let mut pool = SimulationDrivePool::try_new(1, &parent_pool).expect("pool");
        let before = pool.snapshot();
        assert_eq!(
            pool.try_add_drive(&parent_pool, parent[0], child[0], &frames, &dynamics,),
            Err(SimulationDrivePoolError::BodyAccess {
                drive: None,
                endpoint: SimulationDriveEndpoint::Child,
                source: SimulationRigidBodyPoolError::ForeignHandle,
            })
        );
        assert_eq!(pool.snapshot(), before);
    }

    #[test]
    fn add_rejects_a_different_body_pool_before_drive_effects() {
        let (bound_body_pool, body) = bodies(&[4, 4]);
        let (other_body_pool, _) = bodies(&[4, 4]);
        let frames = frames(0x3f80_0000);
        let dynamics = dynamics(0x4000_0000);
        let mut pool = SimulationDrivePool::try_new(1, &bound_body_pool).expect("pool");
        let before = pool.snapshot();
        assert_eq!(
            pool.try_add_drive(&other_body_pool, body[0], body[1], &frames, &dynamics),
            Err(SimulationDrivePoolError::BodyPoolMismatch)
        );
        assert_eq!(pool.snapshot(), before);
    }

    #[test]
    fn full_capacity_returns_none_before_cloning_inputs_or_mutating() {
        let (body_pool, body) = bodies(&[4, 4]);
        let (foreign_body_pool, foreign_body) = bodies(&[4, 4]);
        let first_frames = frames(0x3f00_0000);
        let first_dynamics = dynamics(0x4000_0000);
        let extra_frames = frames(0x4100_0000);
        let extra_dynamics = dynamics(0x4200_0000);
        let mut pool = SimulationDrivePool::try_new(1, &body_pool).expect("pool");
        add(
            &mut pool,
            &body_pool,
            body[0],
            body[1],
            &first_frames,
            &first_dynamics,
        );
        let before = pool.snapshot();
        assert_eq!(Arc::strong_count(&extra_frames), 1);
        assert_eq!(Arc::strong_count(&extra_dynamics), 1);
        assert_eq!(
            pool.try_add_drive(
                &foreign_body_pool,
                foreign_body[1],
                foreign_body[0],
                &extra_frames,
                &extra_dynamics,
            ),
            Ok(None)
        );
        assert_eq!(pool.snapshot(), before);
        assert_eq!(Arc::strong_count(&extra_frames), 1);
        assert_eq!(Arc::strong_count(&extra_dynamics), 1);
    }

    #[test]
    fn removal_preserves_payload_and_reuse_is_fifo() {
        let (body_pool, body) = bodies(&[4, 4]);
        let old_frames = frames(0x3e00_0000);
        let old_dynamics = dynamics(0x3f00_0000);
        let new_frames = frames(0x4000_0000);
        let new_dynamics = dynamics(0x4100_0000);
        let mut pool = SimulationDrivePool::try_new(3, &body_pool).expect("pool");
        let first = add(
            &mut pool,
            &body_pool,
            body[0],
            body[1],
            &old_frames,
            &old_dynamics,
        );
        pool.slots[0].tag = Some(0xdead_beef);
        pool.slots[0].spy = Some(0x1234_5678);
        let before = pool.snapshot().slots[0].clone();
        pool.try_remove_drive(first).expect("remove");
        let removed = &pool.snapshot().slots[0];
        assert_eq!(removed.frames_present, before.frames_present);
        assert_eq!(removed.dynamics_present, before.dynamics_present);
        assert_eq!(removed.body_a, before.body_a);
        assert_eq!(removed.body_b, before.body_b);
        assert_eq!(removed.tag, Some(0xdead_beef));
        assert_eq!(removed.spy, Some(0x1234_5678));
        assert_eq!(removed.occurrence, before.occurrence);
        assert_eq!(
            pool.snapshot().free_order,
            [SimulationDriveId(1), SimulationDriveId(2), first.id()]
        );

        let second = add(
            &mut pool,
            &body_pool,
            body[1],
            body[0],
            &new_frames,
            &new_dynamics,
        );
        assert_eq!(second.id(), SimulationDriveId(1));
        let third = add(
            &mut pool,
            &body_pool,
            body[1],
            body[0],
            &new_frames,
            &new_dynamics,
        );
        assert_eq!(third.id(), SimulationDriveId(2));
        let reused = add(
            &mut pool,
            &body_pool,
            body[1],
            body[0],
            &new_frames,
            &new_dynamics,
        );
        assert_eq!(reused.id(), first.id());
        let slot = &pool.snapshot().slots[0];
        assert_eq!(slot.tag, Some(0xdead_beef), "AddDrive leaves tag untouched");
        assert_eq!(slot.spy, Some(0), "AddDrive resets spy");
        assert_eq!(slot.body_b, Some(body[1].identity()));
        assert_eq!(slot.body_a, Some(body[0].identity()));
    }

    #[test]
    fn removal_splices_active_order_and_readd_appends_at_tail() {
        let (body_pool, body) = bodies(&[4, 4]);
        let frames = frames(0x3f80_0000);
        let dynamics = dynamics(0x4000_0000);
        let mut pool = SimulationDrivePool::try_new(3, &body_pool).expect("pool");
        let first = add(&mut pool, &body_pool, body[0], body[1], &frames, &dynamics);
        let middle = add(&mut pool, &body_pool, body[0], body[1], &frames, &dynamics);
        let last = add(&mut pool, &body_pool, body[0], body[1], &frames, &dynamics);
        assert_eq!(
            pool.snapshot().active_order,
            [first.id(), middle.id(), last.id()]
        );

        pool.try_remove_drive(middle).expect("remove middle");
        assert_eq!(pool.snapshot().active_order, [first.id(), last.id()]);
        let appended = add(&mut pool, &body_pool, body[0], body[1], &frames, &dynamics);
        assert_eq!(appended.id(), middle.id());
        assert_eq!(
            pool.snapshot().active_order,
            [first.id(), last.id(), middle.id()]
        );
    }

    #[test]
    fn removal_does_not_consult_bodies_after_body_first_teardown() {
        let (mut body_pool, body) = bodies(&[4, 4]);
        let frames = frames(0x3f80_0000);
        let dynamics = dynamics(0x4000_0000);
        let mut pool = SimulationDrivePool::try_new(1, &body_pool).expect("pool");
        let drive = add(&mut pool, &body_pool, body[0], body[1], &frames, &dynamics);
        body_pool
            .try_remove_rigid_body(body[0])
            .expect("remove parent first");
        body_pool
            .try_remove_rigid_body(body[1])
            .expect("remove child first");
        pool.try_remove_drive(drive)
            .expect("Drive removal does not dereference bodies");
        assert_eq!(pool.counts(), SimulationDriveCounts { free: 1, active: 0 });
    }

    #[test]
    fn build_order_uses_active_ring_parent_then_child_and_either_bit_four() {
        let (body_pool, body) = bodies(&[1, 1, 4, 1, 1, 4]);
        let frames = frames(0x3f80_0000);
        let dynamics = dynamics(0x4000_0000);
        let mut pool = SimulationDrivePool::try_new(3, &body_pool).expect("pool");
        let skipped = add(&mut pool, &body_pool, body[0], body[1], &frames, &dynamics);
        let parent_active = add(&mut pool, &body_pool, body[2], body[3], &frames, &dynamics);
        let child_active = add(&mut pool, &body_pool, body[4], body[5], &frames, &dynamics);
        let order = pool
            .try_active_build_order(&body_pool)
            .expect("build order");
        assert_eq!(order, [parent_active.identity(), child_active.identity()]);
        assert!(!order.contains(&skipped.identity()));
    }

    #[test]
    fn build_snapshots_skip_rejected_locks_and_detach_admitted_values() {
        let (body_pool, body) = bodies(&[1, 1, 4, 1]);
        let rejected_frames = frames(0x3f00_0000);
        let rejected_dynamics = dynamics(0x3f80_0000);
        let _ = std::panic::catch_unwind({
            let rejected_frames = Arc::clone(&rejected_frames);
            move || {
                let _guard = rejected_frames.write().expect("initial write");
                panic!("poison rejected frames");
            }
        });
        let _ = std::panic::catch_unwind({
            let rejected_dynamics = Arc::clone(&rejected_dynamics);
            move || {
                let _guard = rejected_dynamics.write().expect("initial write");
                panic!("poison rejected dynamics");
            }
        });
        let admitted_frames = frames(0x4000_0000);
        let admitted_dynamics = dynamics(0x4080_0000);
        let expected_frames = *admitted_frames.read().expect("frames");
        let expected_dynamics = *admitted_dynamics.read().expect("dynamics");
        let mut pool = SimulationDrivePool::try_new(2, &body_pool).expect("pool");
        let rejected = add(
            &mut pool,
            &body_pool,
            body[0],
            body[1],
            &rejected_frames,
            &rejected_dynamics,
        );
        let admitted = add(
            &mut pool,
            &body_pool,
            body[2],
            body[3],
            &admitted_frames,
            &admitted_dynamics,
        );
        pool.slots[admitted.id().0 as usize].spy = Some(0xa5a5_5a5a);
        let frames_owner_count = Arc::strong_count(&admitted_frames);
        let dynamics_owner_count = Arc::strong_count(&admitted_dynamics);
        let before = pool.snapshot();

        let snapshots = pool
            .try_active_build_snapshots(&body_pool)
            .expect("rejected locks are not read");

        assert_eq!(snapshots.len(), 1);
        let copied = snapshots[0];
        assert_eq!(copied.drive, admitted.identity());
        assert_ne!(copied.drive, rejected.identity());
        assert_eq!(copied.body_b.observation(), body[2].observation());
        assert_eq!(copied.body_a.observation(), body[3].observation());
        assert_eq!(copied.body_b.state, 4);
        assert_eq!(copied.body_a.state, 1);
        assert_eq!(copied.frames, expected_frames);
        assert_eq!(copied.dynamics, expected_dynamics);
        assert_eq!(copied.time_step.to_bits(), body_pool.timestep().to_bits());
        assert_eq!(copied.spy, 0xa5a5_5a5a);
        assert_eq!(Arc::strong_count(&admitted_frames), frames_owner_count);
        assert_eq!(Arc::strong_count(&admitted_dynamics), dynamics_owner_count);
        assert!(!std::mem::needs_drop::<SimulationDriveBuildSnapshot>());
        assert_eq!(pool.snapshot(), before);
    }

    #[test]
    fn build_snapshots_preserve_dense_order_self_shared_roles_and_live_inputs() {
        let (mut body_pool, body) = bodies(&[4, 1, 4]);
        let time_step = f32::from_bits(0x3c23_d70a);
        body_pool.begin_dynamic_update_frame(time_step);
        let shared_frames = frames(0x3f00_0000);
        let shared_dynamics = dynamics(0x3f80_0000);
        let mut pool = SimulationDrivePool::try_new(3, &body_pool).expect("pool");
        let self_pair = add(
            &mut pool,
            &body_pool,
            body[0],
            body[0],
            &shared_frames,
            &shared_dynamics,
        );
        let shared_parent = add(
            &mut pool,
            &body_pool,
            body[0],
            body[1],
            &shared_frames,
            &shared_dynamics,
        );
        let shared_child = add(
            &mut pool,
            &body_pool,
            body[2],
            body[0],
            &shared_frames,
            &shared_dynamics,
        );
        pool.slots[self_pair.id().0 as usize].spy = Some(0x10);
        pool.slots[shared_parent.id().0 as usize].spy = Some(0x20);
        pool.slots[shared_child.id().0 as usize].spy = Some(0x30);

        let updated_frames = SimulationDriveFrames {
            quaternion_a: [1.0, 2.0, 3.0, 4.0],
            position_a: [5.0, 6.0, 7.0, 8.0],
            quaternion_b: [9.0, 10.0, 11.0, 12.0],
            position_b: [13.0, 14.0, 15.0, 16.0],
        };
        let updated_dynamics = SimulationDriveDynamics {
            linear: SimulationDriveDynamicsParams {
                spring: 17.0,
                damping: 18.0,
                strength: 19.0,
                raw_type: 1,
            },
            angular: SimulationDriveDynamicsParams {
                spring: 20.0,
                damping: 21.0,
                strength: 22.0,
                raw_type: 2,
            },
        };
        *shared_frames.write().expect("write live frames") = updated_frames;
        *shared_dynamics.write().expect("write live dynamics") = updated_dynamics;

        let snapshots = pool
            .try_active_build_snapshots(&body_pool)
            .expect("three admitted snapshots");

        assert_eq!(
            snapshots.iter().map(|item| item.drive).collect::<Vec<_>>(),
            [
                self_pair.identity(),
                shared_parent.identity(),
                shared_child.identity(),
            ]
        );
        assert_eq!(snapshots[0].body_b.observation(), body[0].observation());
        assert_eq!(snapshots[0].body_a.observation(), body[0].observation());
        assert_eq!(snapshots[1].body_b.observation(), body[0].observation());
        assert_eq!(snapshots[1].body_a.observation(), body[1].observation());
        assert_eq!(snapshots[2].body_b.observation(), body[2].observation());
        assert_eq!(snapshots[2].body_a.observation(), body[0].observation());
        for (snapshot, expected_spy) in snapshots.iter().zip([0x10, 0x20, 0x30]) {
            assert_eq!(snapshot.frames, updated_frames);
            assert_eq!(snapshot.dynamics, updated_dynamics);
            assert_eq!(snapshot.time_step.to_bits(), time_step.to_bits());
            assert_eq!(snapshot.spy, expected_spy);
        }
    }

    #[test]
    fn build_snapshot_failure_exposes_no_partial_batch_or_pool_effects() {
        let (body_pool, body) = bodies(&[4, 1, 4, 1]);
        let healthy_frames = frames(0x3f00_0000);
        let healthy_dynamics = dynamics(0x3f80_0000);
        let poisoned_frames = frames(0x4000_0000);
        let poisoned_dynamics = dynamics(0x4080_0000);
        let _ = std::panic::catch_unwind({
            let poisoned_dynamics = Arc::clone(&poisoned_dynamics);
            move || {
                let _guard = poisoned_dynamics.write().expect("initial write");
                panic!("poison admitted dynamics");
            }
        });
        let mut pool = SimulationDrivePool::try_new(2, &body_pool).expect("pool");
        add(
            &mut pool,
            &body_pool,
            body[0],
            body[1],
            &healthy_frames,
            &healthy_dynamics,
        );
        let failed = add(
            &mut pool,
            &body_pool,
            body[2],
            body[3],
            &poisoned_frames,
            &poisoned_dynamics,
        );
        let before_drive = pool.snapshot();
        let before_body = body_pool.snapshot();

        assert_eq!(
            pool.try_active_build_snapshots(&body_pool),
            Err(SimulationDrivePoolError::DynamicsPoisoned {
                slot: failed.id().index(),
            })
        );
        assert_eq!(pool.snapshot(), before_drive);
        assert_eq!(body_pool.snapshot(), before_body);
    }

    #[test]
    fn reused_body_slot_is_rejected_by_occurrence_at_the_parent_endpoint() {
        let (mut body_pool, body) = bodies(&[4, 4]);
        let frames = frames(0x3f80_0000);
        let dynamics = dynamics(0x4000_0000);
        let mut pool = SimulationDrivePool::try_new(1, &body_pool).expect("pool");
        let drive = add(&mut pool, &body_pool, body[0], body[1], &frames, &dynamics);
        body_pool
            .try_remove_rigid_body(body[0])
            .expect("remove parent");
        let replacement = body_pool
            .try_add_rigid_body(identity(), None, 4)
            .expect("replace parent")
            .expect("body capacity");
        assert_eq!(replacement.id(), body[0].id());
        let before = pool.snapshot();
        assert_eq!(
            pool.try_active_build_order(&body_pool),
            Err(SimulationDrivePoolError::BodyAccess {
                drive: Some(drive.id().index()),
                endpoint: SimulationDriveEndpoint::Parent,
                source: SimulationRigidBodyPoolError::StaleHandle { slot: 0 },
            })
        );
        assert_eq!(pool.snapshot(), before);
    }

    #[test]
    fn build_validates_parent_then_reports_a_stale_child_occurrence() {
        let (mut body_pool, body) = bodies(&[4, 4]);
        let frames = frames(0x3f80_0000);
        let dynamics = dynamics(0x4000_0000);
        let mut pool = SimulationDrivePool::try_new(1, &body_pool).expect("pool");
        let drive = add(&mut pool, &body_pool, body[0], body[1], &frames, &dynamics);
        body_pool
            .try_remove_rigid_body(body[1])
            .expect("remove child");
        let replacement = body_pool
            .try_add_rigid_body(identity(), None, 4)
            .expect("replace child")
            .expect("body capacity");
        assert_eq!(replacement.id(), body[1].id());
        let before = pool.snapshot();
        assert_eq!(
            pool.try_active_build_order(&body_pool),
            Err(SimulationDrivePoolError::BodyAccess {
                drive: Some(drive.id().index()),
                endpoint: SimulationDriveEndpoint::Child,
                source: SimulationRigidBodyPoolError::StaleHandle { slot: 1 },
            })
        );
        assert_eq!(pool.snapshot(), before);
    }

    #[test]
    fn foreign_double_and_stale_handles_have_zero_effects() {
        let (body_pool, body) = bodies(&[4, 4]);
        let frames = frames(0x3f80_0000);
        let dynamics = dynamics(0x4000_0000);
        let mut first = SimulationDrivePool::try_new(1, &body_pool).expect("pool");
        let mut second = SimulationDrivePool::try_new(1, &body_pool).expect("pool");
        let old = add(&mut first, &body_pool, body[0], body[1], &frames, &dynamics);
        let before = second.snapshot();
        assert_eq!(
            second.try_remove_drive(old),
            Err(SimulationDrivePoolError::ForeignHandle)
        );
        assert_eq!(second.snapshot(), before);

        first.try_remove_drive(old).expect("remove");
        let removed = first.snapshot();
        assert_eq!(
            first.try_remove_drive(old),
            Err(SimulationDrivePoolError::DriveIsFree { slot: 0 })
        );
        assert_eq!(first.snapshot(), removed);
        let fresh = add(&mut first, &body_pool, body[0], body[1], &frames, &dynamics);
        let reused = first.snapshot();
        assert_eq!(
            first.try_remove_drive(old),
            Err(SimulationDrivePoolError::StaleHandle { slot: 0 })
        );
        assert_eq!(first.snapshot(), reused);
        first.try_remove_drive(fresh).expect("fresh remove");
    }

    #[test]
    fn spy_write_changes_only_owned_live_slot_and_rejects_invalid_handles() {
        let (body_pool, body) = bodies(&[4, 4]);
        let frames = frames(0x3f80_0000);
        let dynamics = dynamics(0x4000_0000);
        let mut pool = SimulationDrivePool::try_new(1, &body_pool).unwrap();
        let old = add(&mut pool, &body_pool, body[0], body[1], &frames, &dynamics);
        let mut expected = pool.snapshot();
        pool.try_set_spy(old, 1).unwrap();
        expected.slots[0].spy = Some(1);
        assert_eq!(pool.snapshot(), expected);

        let mut foreign = SimulationDrivePool::try_new(1, &body_pool).unwrap();
        let untouched = foreign.snapshot();
        assert_eq!(
            foreign.try_set_spy(old, 9),
            Err(SimulationDrivePoolError::ForeignHandle)
        );
        assert_eq!(foreign.snapshot(), untouched);
        pool.try_remove_drive(old).unwrap();
        let removed = pool.snapshot();
        assert_eq!(
            pool.try_set_spy(old, 9),
            Err(SimulationDrivePoolError::DriveIsFree { slot: 0 })
        );
        assert_eq!(pool.snapshot(), removed);
        let fresh = add(&mut pool, &body_pool, body[0], body[1], &frames, &dynamics);
        let reused = pool.snapshot();
        assert_eq!(
            pool.try_set_spy(old, 9),
            Err(SimulationDrivePoolError::StaleHandle { slot: 0 })
        );
        assert_eq!(pool.snapshot(), reused);
        pool.try_set_spy(fresh, 1).unwrap();
        pool.try_remove_drive(fresh).unwrap();
    }

    #[test]
    fn retained_input_owners_survive_caller_drop_and_are_replaced_on_reuse() {
        let (body_pool, body) = bodies(&[4, 4]);
        let initial_frames = frames(0x3f80_0000);
        let initial_dynamics = dynamics(0x4000_0000);
        let old_frames = Arc::downgrade(&initial_frames);
        let old_dynamics = Arc::downgrade(&initial_dynamics);
        let mut pool = SimulationDrivePool::try_new(1, &body_pool).expect("pool");
        let handle = add(
            &mut pool,
            &body_pool,
            body[0],
            body[1],
            &initial_frames,
            &initial_dynamics,
        );
        drop(initial_frames);
        drop(initial_dynamics);
        assert!(old_frames.upgrade().is_some());
        assert!(old_dynamics.upgrade().is_some());
        assert_eq!(
            pool.try_frames(handle.identity())
                .expect("retained frames")
                .quaternion_a[0]
                .to_bits(),
            0x3f80_0000
        );
        assert_eq!(
            pool.try_dynamics(handle.identity())
                .expect("retained dynamics")
                .linear
                .spring
                .to_bits(),
            0x4000_0000
        );

        let old_identity = handle.identity();
        pool.try_remove_drive(handle).expect("remove");
        assert_eq!(
            pool.try_frames(old_identity).expect_err("removed identity"),
            SimulationDrivePoolError::DriveIsFree { slot: 0 }
        );
        let replacement_frames = frames(0x4100_0000);
        let replacement_dynamics = dynamics(0x4200_0000);
        let fresh = add(
            &mut pool,
            &body_pool,
            body[1],
            body[0],
            &replacement_frames,
            &replacement_dynamics,
        );
        assert_eq!(fresh.id(), handle.id());
        assert!(old_frames.upgrade().is_none());
        assert!(old_dynamics.upgrade().is_none());
        assert_eq!(
            pool.try_frames(old_identity).expect_err("stale identity"),
            SimulationDrivePoolError::StaleHandle { slot: 0 }
        );
        assert_eq!(
            frame_bits(pool.try_frames(fresh.identity()).expect("frames")),
            frame_bits(*replacement_frames.read().expect("replacement frames"))
        );
    }

    #[test]
    fn final_occurrence_is_issued_once_then_exhaustion_is_atomic() {
        let (body_pool, body) = bodies(&[4, 4]);
        let frames = frames(0x3f80_0000);
        let dynamics = dynamics(0x4000_0000);
        let mut pool = SimulationDrivePool::try_new(1, &body_pool).expect("pool");
        pool.next_occurrence = NonZeroU64::new(u64::MAX);
        let handle = add(&mut pool, &body_pool, body[0], body[1], &frames, &dynamics);
        assert_eq!(handle.occurrence.get(), u64::MAX);
        pool.try_remove_drive(handle).expect("remove");
        let before = pool.snapshot();
        assert_eq!(
            pool.try_add_drive(&body_pool, body[0], body[1], &frames, &dynamics),
            Err(SimulationDrivePoolError::OccurrenceExhausted)
        );
        assert_eq!(pool.snapshot(), before);
    }

    #[test]
    fn corrupt_rings_are_rejected_before_add_or_remove_effects() {
        let (body_pool, body) = bodies(&[4, 4]);
        let frames = frames(0x3f80_0000);
        let dynamics = dynamics(0x4000_0000);
        let mut add_pool = SimulationDrivePool::try_new(2, &body_pool).expect("pool");
        add_pool.slots[0].links.right = SimulationDriveNode::Drive(SimulationDriveId(0));
        add_pool.validated_topology.take(); // Test-only corruption bypasses ring mutators.
        let before = add_pool.snapshot();
        assert_eq!(
            add_pool.try_add_drive(&body_pool, body[0], body[1], &frames, &dynamics),
            Err(SimulationDrivePoolError::CorruptStructure)
        );
        assert_eq!(add_pool.snapshot(), before);

        let mut remove_pool = SimulationDrivePool::try_new(2, &body_pool).expect("pool");
        let handle = add(
            &mut remove_pool,
            &body_pool,
            body[0],
            body[1],
            &frames,
            &dynamics,
        );
        remove_pool.slots[1].links.left = SimulationDriveNode::Drive(handle.id());
        remove_pool.validated_topology.take(); // Test-only corruption bypasses ring mutators.
        let before = remove_pool.snapshot();
        assert_eq!(
            remove_pool.try_remove_drive(handle),
            Err(SimulationDrivePoolError::CorruptStructure)
        );
        assert_eq!(remove_pool.snapshot(), before);
    }

    #[test]
    fn authority_handles_and_pool_have_required_thread_and_drop_traits() {
        fn assert_send_sync_static<T: Send + Sync + 'static>() {}
        assert_send_sync_static::<SimulationDrivePool>();
        assert_send_sync_static::<SimulationDriveHandle>();
        assert!(!std::mem::needs_drop::<SimulationDriveHandle>());
    }

    #[test]
    fn local_authority_allocator_is_checked_and_never_wraps() {
        let source = AtomicU64::new(u64::MAX - 1);
        assert_eq!(
            fresh_drive_pool_authority(&source),
            Ok(SimulationDrivePoolAuthority(u64::MAX - 1))
        );
        assert_eq!(
            fresh_drive_pool_authority(&source),
            Err(SimulationDrivePoolError::AuthorityExhausted)
        );
        assert_eq!(source.load(Ordering::Relaxed), u64::MAX);
    }
}
