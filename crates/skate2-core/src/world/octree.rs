//! Concrete local collision Octree behavior from the supplied debug build.
//!
//! This module covers resource planning, initialization, sequential insertion,
//! and the matching sequential removal phase. It deliberately does not expose
//! the unaudited spatial-query API. The reference accepts arbitrary entry
//! indices without checking them; the stream-file caller instead supplies each
//! compact index once, in ascending order to both `Insert` and `Remove`, so
//! [`LocalOctree::insert_next`] and [`LocalOctree::remove_next`] make that
//! proven contract explicit.

use core::fmt;

/// Sentinel used by the reference's packed node and entry links.
pub const NO_PACKED_LINK: u16 = 0xffff;

const OCTREE_HEADER_SIZE: usize = 0x40;
const OCTREE_ALIGNMENT: usize = 0x20;
const AABB_SIZE: usize = 0x20;
const NODE_SIZE: usize = 0x20;
const ENTRY_SIZE: usize = 0x08;
const CHILD_COUNT: usize = 8;
const CHILD_SENTINEL: u8 = 0xff;
const SPLIT_PUSH_THRESHOLD: u8 = 3;
const OVERLAP_SCALE: f32 = f32::from_bits(0xbecc_cccd); // -0.4
const HALF: f32 = 0.5;

/// Maximum entry count compatible with the native `u16` links and their
/// reserved `0xffff` sentinel.
pub const MAX_OCTREE_ENTRIES: usize = 65_535;

/// A four-lane, raw-preserving `rw::collision::AABBox` value.
///
/// The PDB type is 0x20 bytes: a four-lane minimum followed by a four-lane
/// maximum. Only xyz participate in child selection, but the constructor and
/// bbox copy retain the fourth lane.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AabbWords {
    pub min: [u32; 4],
    pub max: [u32; 4],
}

impl AabbWords {
    #[must_use]
    pub fn from_f32(min: [f32; 4], max: [f32; 4]) -> Self {
        Self {
            min: min.map(f32::to_bits),
            max: max.map(f32::to_bits),
        }
    }

    #[must_use]
    pub fn min_f32(self) -> [f32; 4] {
        self.min.map(f32::from_bits)
    }

    #[must_use]
    pub fn max_f32(self) -> [f32; 4] {
        self.max.map(f32::from_bits)
    }
}

/// Checked number of entry slots reserved for a local Octree.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OctreeCapacity(u16);

impl OctreeCapacity {
    /// Checks the packed-link limit before descriptor arithmetic or allocation.
    ///
    /// # Errors
    ///
    /// Returns [`CapacityError`] when `requested` would make `0xffff` a valid
    /// entry index instead of the reserved sentinel.
    pub fn new(requested: usize) -> Result<Self, CapacityError> {
        let packed = u16::try_from(requested).map_err(|_| CapacityError {
            requested,
            maximum: MAX_OCTREE_ENTRIES,
        })?;
        Ok(Self(packed))
    }

    #[must_use]
    pub const fn get(self) -> usize {
        self.0 as usize
    }
}

/// A requested Octree capacity cannot be represented without colliding with
/// the native link sentinel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CapacityError {
    pub requested: usize,
    pub maximum: usize,
}

impl fmt::Display for CapacityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Octree capacity {} exceeds packed-link maximum {}",
            self.requested, self.maximum
        )
    }
}

impl std::error::Error for CapacityError {}

/// One of the five resource descriptor slots used by `rw::Resource`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceSlotPlan {
    pub size: usize,
    pub alignment: usize,
}

/// Checked contiguous layout returned by `Octree::GetResourceDescriptor`.
///
/// Reference: `rw::collision::Octree::GetResourceDescriptor` at
/// `0x82adbee8..0x82adbf50`. Only slot zero owns storage; slots one through four
/// are empty with alignment one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OctreeResourcePlan {
    pub resources: [ResourceSlotPlan; 5],
    pub bbox_offset: usize,
    pub node_offset: usize,
    pub entry_offset: usize,
    pub node_count: usize,
}

/// Checked layout arithmetic failed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourcePlanOverflow;

impl fmt::Display for ResourcePlanOverflow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Octree resource-plan arithmetic overflowed")
    }
}

impl std::error::Error for ResourcePlanOverflow {}

impl OctreeResourcePlan {
    /// Computes the exact five-slot descriptor and internal offsets.
    ///
    /// # Errors
    ///
    /// Returns [`ResourcePlanOverflow`] if the target's address-size arithmetic
    /// cannot represent the contiguous resource plan.
    pub fn for_capacity(capacity: OctreeCapacity) -> Result<Self, ResourcePlanOverflow> {
        let entries = capacity.get();
        let node_count = entries / 2 + 1;
        let bbox_offset = OCTREE_HEADER_SIZE;
        let node_offset = bbox_offset
            .checked_add(AABB_SIZE.checked_mul(entries).ok_or(ResourcePlanOverflow)?)
            .ok_or(ResourcePlanOverflow)?;
        let entry_offset = node_offset
            .checked_add(
                NODE_SIZE
                    .checked_mul(node_count)
                    .ok_or(ResourcePlanOverflow)?,
            )
            .ok_or(ResourcePlanOverflow)?;
        let total_size = entry_offset
            .checked_add(
                ENTRY_SIZE
                    .checked_mul(entries)
                    .ok_or(ResourcePlanOverflow)?,
            )
            .ok_or(ResourcePlanOverflow)?;

        Ok(Self {
            resources: [
                ResourceSlotPlan {
                    size: total_size,
                    alignment: OCTREE_ALIGNMENT,
                },
                ResourceSlotPlan {
                    size: 0,
                    alignment: 1,
                },
                ResourceSlotPlan {
                    size: 0,
                    alignment: 1,
                },
                ResourceSlotPlan {
                    size: 0,
                    alignment: 1,
                },
                ResourceSlotPlan {
                    size: 0,
                    alignment: 1,
                },
            ],
            bbox_offset,
            node_offset,
            entry_offset,
            node_count,
        })
    }
}

/// A packed native node/entry link. `0xffff` represents no link.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PackedLink(u16);

impl PackedLink {
    pub const NONE: Self = Self(NO_PACKED_LINK);

    #[must_use]
    pub const fn from_index(index: u16) -> Option<Self> {
        if index == NO_PACKED_LINK {
            None
        } else {
            Some(Self(index))
        }
    }

    #[must_use]
    pub const fn raw(self) -> u16 {
        self.0
    }

    #[must_use]
    pub const fn index(self) -> Option<usize> {
        if self.0 == NO_PACKED_LINK {
            None
        } else {
            Some(self.0 as usize)
        }
    }
}

/// Sequential entry identifier. The sentinel value is unrepresentable.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EntryId(u16);

impl EntryId {
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    #[must_use]
    pub const fn raw(self) -> u16 {
        self.0
    }
}

/// PDB-shaped semantic fields of `rw::collision::Octree::Node`.
///
/// This is not an ABI-layout claim. In particular, free native nodes have only
/// their `m_parent` link initialized; [`NodeSlot`] preserves that distinction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Node {
    pub parent: PackedLink,
    pub child_of_parent: u16,
    pub stuck_entries: PackedLink,
    pub child_types: u16,
    pub child_refs: [PackedLink; CHILD_COUNT],
    pub push_counts: [u8; CHILD_COUNT],
}

impl Node {
    fn initialized(parent: PackedLink, child_of_parent: u8) -> Self {
        Self {
            parent,
            child_of_parent: u16::from(child_of_parent),
            stuck_entries: PackedLink::NONE,
            child_types: 0x00ff,
            child_refs: [PackedLink::NONE; CHILD_COUNT],
            push_counts: [0; CHILD_COUNT],
        }
    }

    fn child_is_leaf(&self, child: u8) -> bool {
        self.child_types & (1_u16 << child) != 0
    }
}

/// Initialized-state distinction for the fixed native node pool.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodeSlot {
    Active(Node),
    /// Only the native `m_parent` storage is initialized while on the free list.
    Free {
        next: PackedLink,
    },
}

/// PDB-shaped semantic fields of `rw::collision::Octree::Entry`.
///
/// This is not an ABI-layout claim.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Entry {
    pub group: u16,
    pub next: PackedLink,
    pub node: PackedLink,
    pub child: u8,
    pub corner: u8,
}

/// The fixed storage is full. The safe API reports this before writing beyond
/// the bbox allocation; the unchecked reference has no corresponding error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OctreeFull {
    pub capacity: usize,
}

impl fmt::Display for OctreeFull {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Octree capacity {} is full", self.capacity)
    }
}

impl std::error::Error for OctreeFull {}

/// A sequential insertion cannot be performed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OctreeInsertError {
    /// All storage described by the resource plan is occupied.
    Full(OctreeFull),
    /// A successful removal has begun the monotonic removal phase.
    RemovalStarted { next_removal: usize },
}

impl fmt::Display for OctreeInsertError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full(error) => error.fmt(formatter),
            Self::RemovalStarted { next_removal } => write!(
                formatter,
                "Octree removal has started at sequential index {next_removal}"
            ),
        }
    }
}

impl std::error::Error for OctreeInsertError {}

/// Observable result of one sequential removal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OctreeRemoval {
    pub entry: EntryId,
    /// The one non-root node returned to the free list, if any.
    pub collapsed_node: Option<u16>,
}

/// A malformed or unavailable sequential removal was rejected before mutation.
///
/// The native routine assumes all of these invariants and can read out of
/// bounds, loop forever, or corrupt storage when they do not hold. Reporting a
/// typed error atomically is an intentional high-confidence safe deviation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OctreeRemovalError {
    NoInsertedEntry {
        next_removal: usize,
        inserted: usize,
    },
    EntryIndexOutOfRange {
        entry: usize,
    },
    MissingEntryNode {
        entry: usize,
    },
    NodeOutOfRange {
        entry: usize,
        node: usize,
    },
    NodeIsFree {
        entry: usize,
        node: usize,
    },
    ChildOutOfRange {
        entry: usize,
        child: u8,
    },
    EntryInInternalChild {
        entry: usize,
        node: usize,
        child: u8,
    },
    ListLinkOutOfRange {
        entry: usize,
        link: usize,
    },
    ListCycle {
        entry: usize,
        at: usize,
    },
    ListContainsRemovedEntry {
        entry: usize,
        listed: usize,
    },
    ListMembershipMismatch {
        entry: usize,
        listed: usize,
    },
    TargetNotInSelectedList {
        entry: usize,
    },
    PushCountUnderflow {
        entry: usize,
        node: usize,
        child: u8,
    },
    MissingParent {
        node: usize,
    },
    ParentOutOfRange {
        node: usize,
        parent: usize,
    },
    ParentIsFree {
        node: usize,
        parent: usize,
    },
    ParentChildOutOfRange {
        node: usize,
        child: u16,
    },
    ParentLinkMismatch {
        node: usize,
        parent: usize,
        child: u8,
    },
    FreeListOutOfRange {
        head: usize,
    },
    FreeListHeadIsActive {
        head: usize,
    },
}

impl fmt::Display for OctreeRemovalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid Octree removal state: {self:?}")
    }
}

impl std::error::Error for OctreeRemovalError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum UnlinkLocation {
    StuckHead,
    ChildHead(u8),
    EntryNext(usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CollapsePlan {
    node: usize,
    node_link: PackedLink,
    parent: usize,
    parent_child: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RemovalPlan {
    entry: EntryId,
    node: usize,
    target_next: PackedLink,
    unlink: UnlinkLocation,
    push_count: Option<(u8, u8)>,
    collapse: Option<CollapsePlan>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ListRemovalPlan {
    unlink: UnlinkLocation,
    list_len: usize,
    corner_count: u8,
}

/// Initialized local `rw::collision::Octree` state.
///
/// Reference: private constructor `0x82adbf58..0x82adc0c0`, `Insert` at
/// `0x82ad6af0..0x82ad6cc0`, and `SplitLeaf` at
/// `0x82ad68e0..0x82ad6ae8`.
///
/// Floating operations retain the recovered expression order. That is
/// high-confidence equivalent for ordinary finite inputs; exceptional PPC/VMX
/// bit parity (NaNs, infinities, signed zero, and denormals) remains unknown.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalOctree {
    extent: AabbWords,
    max_entries: u32,
    max_nodes: u32,
    node_free_list: PackedLink,
    nodes: Vec<NodeSlot>,
    entries: Vec<Entry>,
    bboxes: Vec<AabbWords>,
    next_removal: usize,
}

impl LocalOctree {
    /// Initializes the normalized extent, root, and fixed free-node chain.
    #[must_use]
    pub fn new(capacity: OctreeCapacity, extent: AabbWords) -> Self {
        let max_entries = capacity.get();
        let max_nodes_packed = capacity.0 / 2 + 1;
        let max_nodes = usize::from(max_nodes_packed);
        let mut nodes = Vec::with_capacity(max_nodes);
        nodes.push(NodeSlot::Active(Node::initialized(PackedLink(0), 0)));

        for index in 1..max_nodes_packed {
            let next = if index + 1 < max_nodes_packed {
                PackedLink(index + 1)
            } else {
                PackedLink::NONE
            };
            nodes.push(NodeSlot::Free { next });
        }

        // The native free-list terminator write targets node zero when there
        // is no free node, overwriting the root's parent field.
        if max_nodes == 1 {
            Self::active_node_mut_in(&mut nodes, 0).parent = PackedLink::NONE;
        }

        Self {
            extent: normalize_extent(extent),
            max_entries: u32::from(capacity.0),
            max_nodes: u32::from(max_nodes_packed),
            node_free_list: PackedLink(1),
            nodes,
            entries: Vec::with_capacity(max_entries),
            bboxes: Vec::with_capacity(max_entries),
            next_removal: 0,
        }
    }

    /// Copies one bbox and inserts its ascending compact entry index.
    ///
    /// The bbox append occurs before classification, matching the first store
    /// in `Octree::Insert` at `0x82ad6b00..0x82ad6b34`.
    ///
    /// # Errors
    ///
    /// Returns [`OctreeInsertError`] before changing state when all reserved
    /// entry slots are occupied or a successful removal has begun the
    /// monotonic removal phase.
    ///
    /// # Panics
    ///
    /// Panics only if this module's private node/list invariants have been
    /// violated. Such a state cannot be constructed through the public API.
    pub fn insert_next(&mut self, bbox: AabbWords) -> Result<EntryId, OctreeInsertError> {
        if self.next_removal != 0 {
            return Err(OctreeInsertError::RemovalStarted {
                next_removal: self.next_removal,
            });
        }
        if self.entries.len() >= self.max_entries as usize {
            return Err(OctreeInsertError::Full(OctreeFull {
                capacity: self.max_entries as usize,
            }));
        }

        let entry_index = self.entries.len();
        let Ok(entry_raw) = u16::try_from(entry_index) else {
            return Err(OctreeInsertError::Full(OctreeFull {
                capacity: self.max_entries as usize,
            }));
        };
        let entry_link = PackedLink(entry_raw);
        self.bboxes.push(bbox);

        let mut node_index = 0_usize;
        let mut node_box = self.extent;

        if !bbox_is_inside_root(node_box, bbox) {
            self.add_stuck_entry(entry_link, node_index);
            return Ok(EntryId(entry_raw));
        }

        loop {
            let Some(child) = find_child_containing_bbox(node_box, bbox) else {
                self.add_stuck_entry(entry_link, node_index);
                return Ok(EntryId(entry_raw));
            };

            if self.active_node(node_index).child_is_leaf(child) {
                let selected_box = child_bbox(node_box, child);
                let corner = bbox_in_corner(selected_box, bbox);
                self.add_entry_to_leaf(entry_link, node_index, child, corner);
                if self.active_node(node_index).push_counts[usize::from(child)]
                    > SPLIT_PUSH_THRESHOLD
                    && self.node_free_list != PackedLink::NONE
                {
                    self.split_leaf(node_index, child, selected_box);
                }
                return Ok(EntryId(entry_raw));
            }

            node_index = self.active_node(node_index).child_refs[usize::from(child)]
                .index()
                .expect("an internal child has a node reference");
            node_box = child_bbox(node_box, child);
        }
    }

    /// Removes the next ascending compact entry without clearing its entry or
    /// bbox storage.
    ///
    /// Reference: `rw::collision::Octree::Remove` at
    /// `0x82acc210..0x82acc35c` and `Node::DecPushCount` at
    /// `0x82acc120..0x82acc188`. The state-5 stream caller invokes `Remove`
    /// for compact indices `0..mNumOctreeInfo` at `0x824e1420..0x824e1438`.
    ///
    /// A corner count below `0xff` is decremented before unlinking. At
    /// saturation, the reference instead recounts corner entries before the
    /// unlink, caps the recount at `0xff`, and stores that pre-unlink value.
    /// Empty non-root nodes collapse only once; the newly-empty parent is not
    /// considered recursively.
    ///
    /// # Errors
    ///
    /// Returns [`OctreeRemovalError`] without any logical state change when no
    /// inserted entry remains or when a native unchecked invariant is invalid.
    ///
    /// # Panics
    ///
    /// Panics only if the already-validated private plan and the immediately
    /// following mutation are made inconsistent inside this module. No public
    /// input can alter the tree between those two steps.
    pub fn remove_next(&mut self) -> Result<OctreeRemoval, OctreeRemovalError> {
        let plan = self.plan_removal()?;

        if let Some((child, count)) = plan.push_count {
            self.active_node_mut(plan.node).push_counts[usize::from(child)] = count;
        }

        match plan.unlink {
            UnlinkLocation::StuckHead => {
                self.active_node_mut(plan.node).stuck_entries = plan.target_next;
            }
            UnlinkLocation::ChildHead(child) => {
                self.active_node_mut(plan.node).child_refs[usize::from(child)] = plan.target_next;
            }
            UnlinkLocation::EntryNext(predecessor) => {
                self.entries[predecessor].next = plan.target_next;
            }
        }

        let collapsed_node = plan.collapse.map(|collapse| {
            let parent = self.active_node_mut(collapse.parent);
            parent.child_refs[usize::from(collapse.parent_child)] = PackedLink::NONE;
            parent.push_counts[usize::from(collapse.parent_child)] = 0;
            parent.child_types |= 1_u16 << collapse.parent_child;

            let previous_free_head = self.node_free_list;
            self.nodes[collapse.node] = NodeSlot::Free {
                next: previous_free_head,
            };
            self.node_free_list = collapse.node_link;
            collapse.node_link.raw()
        });

        self.next_removal += 1;
        Ok(OctreeRemoval {
            entry: plan.entry,
            collapsed_node,
        })
    }

    fn plan_removal(&self) -> Result<RemovalPlan, OctreeRemovalError> {
        let entry_index = self.next_removal;
        let Some(target) = self.entries.get(entry_index).copied() else {
            return Err(OctreeRemovalError::NoInsertedEntry {
                next_removal: entry_index,
                inserted: self.entries.len(),
            });
        };
        let entry = EntryId(
            u16::try_from(entry_index)
                .map_err(|_| OctreeRemovalError::EntryIndexOutOfRange { entry: entry_index })?,
        );
        let Some(node_index) = target.node.index() else {
            return Err(OctreeRemovalError::MissingEntryNode { entry: entry_index });
        };
        let Some(node_slot) = self.nodes.get(node_index) else {
            return Err(OctreeRemovalError::NodeOutOfRange {
                entry: entry_index,
                node: node_index,
            });
        };
        let NodeSlot::Active(node) = node_slot else {
            return Err(OctreeRemovalError::NodeIsFree {
                entry: entry_index,
                node: node_index,
            });
        };

        let (head, expected_child, head_location) = if target.child == CHILD_SENTINEL {
            (
                node.stuck_entries,
                CHILD_SENTINEL,
                UnlinkLocation::StuckHead,
            )
        } else {
            if usize::from(target.child) >= CHILD_COUNT {
                return Err(OctreeRemovalError::ChildOutOfRange {
                    entry: entry_index,
                    child: target.child,
                });
            }
            if !node.child_is_leaf(target.child) {
                return Err(OctreeRemovalError::EntryInInternalChild {
                    entry: entry_index,
                    node: node_index,
                    child: target.child,
                });
            }
            (
                node.child_refs[usize::from(target.child)],
                target.child,
                UnlinkLocation::ChildHead(target.child),
            )
        };

        let list =
            self.plan_list_removal(entry_index, node_index, expected_child, head, head_location)?;

        let push_count = if target.child != CHILD_SENTINEL && target.corner != 0 {
            let stored = node.push_counts[usize::from(target.child)];
            let updated = if stored == u8::MAX {
                list.corner_count
            } else {
                stored
                    .checked_sub(1)
                    .ok_or(OctreeRemovalError::PushCountUnderflow {
                        entry: entry_index,
                        node: node_index,
                        child: target.child,
                    })?
            };
            Some((target.child, updated))
        } else {
            None
        };

        let selected_becomes_empty = list.list_len == 1;
        let stuck_empty = if target.child == CHILD_SENTINEL {
            selected_becomes_empty
        } else {
            node.stuck_entries == PackedLink::NONE
        };
        let children_empty_and_leaf = (0_u8..8).all(|child| {
            node.child_is_leaf(child)
                && if target.child == child {
                    selected_becomes_empty
                } else {
                    node.child_refs[usize::from(child)] == PackedLink::NONE
                }
        });

        let collapse = if node_index != 0 && stuck_empty && children_empty_and_leaf {
            Some(self.plan_collapse(node_index, node)?)
        } else {
            None
        };

        Ok(RemovalPlan {
            entry,
            node: node_index,
            target_next: target.next,
            unlink: list.unlink,
            push_count,
            collapse,
        })
    }

    fn plan_list_removal(
        &self,
        entry_index: usize,
        node_index: usize,
        expected_child: u8,
        head: PackedLink,
        head_location: UnlinkLocation,
    ) -> Result<ListRemovalPlan, OctreeRemovalError> {
        let mut visited = vec![false; self.entries.len()];
        let mut cursor = head;
        let mut predecessor = None;
        let mut target_predecessor = None;
        let mut list_len = 0_usize;
        let mut corner_count = 0_u8;
        while let Some(listed_index) = cursor.index() {
            let Some(listed) = self.entries.get(listed_index) else {
                return Err(OctreeRemovalError::ListLinkOutOfRange {
                    entry: entry_index,
                    link: listed_index,
                });
            };
            if visited[listed_index] {
                return Err(OctreeRemovalError::ListCycle {
                    entry: entry_index,
                    at: listed_index,
                });
            }
            visited[listed_index] = true;
            if listed_index < self.next_removal {
                return Err(OctreeRemovalError::ListContainsRemovedEntry {
                    entry: entry_index,
                    listed: listed_index,
                });
            }
            if listed.node.index() != Some(node_index) || listed.child != expected_child {
                return Err(OctreeRemovalError::ListMembershipMismatch {
                    entry: entry_index,
                    listed: listed_index,
                });
            }
            if listed_index == entry_index {
                target_predecessor = predecessor;
            }
            if listed.corner != 0 {
                corner_count = corner_count.saturating_add(1);
            }
            list_len += 1;
            predecessor = Some(listed_index);
            cursor = listed.next;
        }

        let unlink = match target_predecessor {
            Some(predecessor) => UnlinkLocation::EntryNext(predecessor),
            None if head.index() == Some(entry_index) => head_location,
            None => {
                return Err(OctreeRemovalError::TargetNotInSelectedList { entry: entry_index });
            }
        };

        Ok(ListRemovalPlan {
            unlink,
            list_len,
            corner_count,
        })
    }

    fn plan_collapse(
        &self,
        node_index: usize,
        node: &Node,
    ) -> Result<CollapsePlan, OctreeRemovalError> {
        let Some(parent_index) = node.parent.index() else {
            return Err(OctreeRemovalError::MissingParent { node: node_index });
        };
        let Some(parent_slot) = self.nodes.get(parent_index) else {
            return Err(OctreeRemovalError::ParentOutOfRange {
                node: node_index,
                parent: parent_index,
            });
        };
        let NodeSlot::Active(parent) = parent_slot else {
            return Err(OctreeRemovalError::ParentIsFree {
                node: node_index,
                parent: parent_index,
            });
        };
        let Ok(parent_child) = u8::try_from(node.child_of_parent) else {
            return Err(OctreeRemovalError::ParentChildOutOfRange {
                node: node_index,
                child: node.child_of_parent,
            });
        };
        if usize::from(parent_child) >= CHILD_COUNT {
            return Err(OctreeRemovalError::ParentChildOutOfRange {
                node: node_index,
                child: node.child_of_parent,
            });
        }
        let expected_node = PackedLink(
            u16::try_from(node_index).expect("validated active node index fits packed link"),
        );
        if parent_index == node_index
            || parent.child_is_leaf(parent_child)
            || parent.child_refs[usize::from(parent_child)] != expected_node
        {
            return Err(OctreeRemovalError::ParentLinkMismatch {
                node: node_index,
                parent: parent_index,
                child: parent_child,
            });
        }

        if let Some(free_head) = self.node_free_list.index() {
            let Some(free_slot) = self.nodes.get(free_head) else {
                return Err(OctreeRemovalError::FreeListOutOfRange { head: free_head });
            };
            if matches!(free_slot, NodeSlot::Active(_)) {
                return Err(OctreeRemovalError::FreeListHeadIsActive { head: free_head });
            }
        }

        Ok(CollapsePlan {
            node: node_index,
            node_link: expected_node,
            parent: parent_index,
            parent_child,
        })
    }

    /// Returns an initialized node, or `None` for an out-of-range/free slot.
    #[must_use]
    pub fn node(&self, index: usize) -> Option<&Node> {
        match self.nodes.get(index)? {
            NodeSlot::Active(node) => Some(node),
            NodeSlot::Free { .. } => None,
        }
    }

    #[must_use]
    pub const fn extent(&self) -> AabbWords {
        self.extent
    }

    #[must_use]
    pub const fn max_entries(&self) -> u32 {
        self.max_entries
    }

    #[must_use]
    pub const fn max_nodes(&self) -> u32 {
        self.max_nodes
    }

    #[must_use]
    pub const fn node_free_list(&self) -> PackedLink {
        self.node_free_list
    }

    /// Next compact index expected by [`Self::remove_next`].
    #[must_use]
    pub const fn next_removal(&self) -> usize {
        self.next_removal
    }

    #[must_use]
    pub fn node_slots(&self) -> &[NodeSlot] {
        &self.nodes
    }

    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    #[must_use]
    pub fn bboxes(&self) -> &[AabbWords] {
        &self.bboxes
    }

    fn active_node(&self, index: usize) -> &Node {
        match &self.nodes[index] {
            NodeSlot::Active(node) => node,
            NodeSlot::Free { .. } => panic!("requested node is still free"),
        }
    }

    fn active_node_mut(&mut self, index: usize) -> &mut Node {
        Self::active_node_mut_in(&mut self.nodes, index)
    }

    fn active_node_mut_in(nodes: &mut [NodeSlot], index: usize) -> &mut Node {
        match &mut nodes[index] {
            NodeSlot::Active(node) => node,
            NodeSlot::Free { .. } => panic!("requested node is still free"),
        }
    }

    fn add_stuck_entry(&mut self, entry: PackedLink, node_index: usize) {
        let previous = self.active_node(node_index).stuck_entries;
        self.entries.push(Entry {
            group: 0,
            next: previous,
            node: PackedLink::from_index(
                u16::try_from(node_index).expect("checked node index fits packed link"),
            )
            .expect("active node cannot use sentinel index"),
            child: CHILD_SENTINEL,
            corner: 0,
        });
        self.active_node_mut(node_index).stuck_entries = entry;
    }

    fn add_entry_to_leaf(&mut self, entry: PackedLink, node_index: usize, child: u8, corner: bool) {
        let child_index = usize::from(child);
        let previous = self.active_node(node_index).child_refs[child_index];
        self.entries.push(Entry {
            group: 0,
            next: previous,
            node: PackedLink::from_index(
                u16::try_from(node_index).expect("checked node index fits packed link"),
            )
            .expect("active node cannot use sentinel index"),
            child,
            corner: u8::from(corner),
        });

        let node = self.active_node_mut(node_index);
        node.child_refs[child_index] = entry;
        if corner {
            node.push_counts[child_index] = node.push_counts[child_index].saturating_add(1);
        }
    }

    fn take_free_node(&mut self, parent: usize, child: u8) -> Option<usize> {
        let free_index = self.node_free_list.index()?;
        let next = match self.nodes.get(free_index)? {
            NodeSlot::Free { next } => *next,
            NodeSlot::Active(_) => return None,
        };
        self.node_free_list = next;
        let parent_link = PackedLink::from_index(
            u16::try_from(parent).expect("checked parent index fits packed link"),
        )
        .expect("active parent cannot use sentinel index");
        self.nodes[free_index] = NodeSlot::Active(Node::initialized(parent_link, child));
        Some(free_index)
    }

    fn split_leaf(&mut self, parent_index: usize, parent_child: u8, node_box: AabbWords) {
        let Some(new_node_index) = self.take_free_node(parent_index, parent_child) else {
            return;
        };
        let old_head = self.active_node(parent_index).child_refs[usize::from(parent_child)];
        let new_node_link = PackedLink::from_index(
            u16::try_from(new_node_index).expect("checked new-node index fits packed link"),
        )
        .expect("allocated node cannot use sentinel index");
        {
            let parent = self.active_node_mut(parent_index);
            parent.child_refs[usize::from(parent_child)] = new_node_link;
            parent.child_types &= !(1_u16 << parent_child);
        }

        let mut cursor = old_head;
        while let Some(entry_index) = cursor.index() {
            let next = self.entries[entry_index].next;
            let bbox = self.bboxes[entry_index];
            if self.entries[entry_index].corner != 0 {
                let child = find_child_containing_bbox(node_box, bbox)
                    .expect("a corner-eligible entry has a containing child");
                let child_index = usize::from(child);
                let destination = self.active_node(new_node_index).child_refs[child_index];
                let selected_box = child_bbox(node_box, child);
                let corner = bbox_in_corner(selected_box, bbox);
                self.entries[entry_index].next = destination;
                self.entries[entry_index].node = new_node_link;
                self.entries[entry_index].child = child;
                self.entries[entry_index].corner = u8::from(corner);
                let node = self.active_node_mut(new_node_index);
                node.child_refs[child_index] = cursor;
                if corner {
                    node.push_counts[child_index] = node.push_counts[child_index].saturating_add(1);
                }
            } else {
                let destination = self.active_node(new_node_index).stuck_entries;
                self.entries[entry_index].next = destination;
                self.entries[entry_index].node = new_node_link;
                self.entries[entry_index].child = CHILD_SENTINEL;
                self.entries[entry_index].corner = 0;
                self.active_node_mut(new_node_index).stuck_entries = cursor;
            }
            cursor = next;
        }
    }
}

fn normalize_extent(extent: AabbWords) -> AabbWords {
    let input_min = extent.min_f32();
    let input_max = extent.max_f32();
    let mut half = [0.0; 4];
    let mut center = [0.0; 4];
    for lane in 0..4 {
        half[lane] = (input_max[lane] - input_min[lane]) * HALF;
        center[lane] = (input_max[lane] + input_min[lane]) * HALF;
    }
    let xy = select_larger(half[0], half[1]);
    let radius = select_larger(xy, half[2]);
    let uniform_half = [radius, radius, radius, 0.0];
    let mut minimum = [0.0; 4];
    let mut maximum = [0.0; 4];
    for lane in 0..4 {
        minimum[lane] = center[lane] - uniform_half[lane];
        maximum[lane] = center[lane] + uniform_half[lane];
    }
    AabbWords::from_f32(minimum, maximum)
}

fn select_larger(first: f32, second: f32) -> f32 {
    let difference = first - second;
    if difference >= 0.0 { first } else { second }
}

fn bbox_is_inside_root(root: AabbWords, bbox: AabbWords) -> bool {
    let root_min = root.min_f32();
    let root_max = root.max_f32();
    let bbox_min = bbox.min_f32();
    let bbox_max = bbox.max_f32();
    (0..3).all(|axis| bbox_max[axis] <= root_max[axis] && root_min[axis] <= bbox_min[axis])
}

fn overlap_bounds(node_box: AabbWords) -> ([f32; 4], [f32; 4]) {
    let minimum = node_box.min_f32();
    let maximum = node_box.max_f32();
    let mut lower = [0.0; 4];
    let mut upper = [0.0; 4];
    for lane in 0..4 {
        let size = maximum[lane] - minimum[lane];
        let scale = if lane < 3 { OVERLAP_SCALE } else { 0.0 };
        let margin = size * scale;
        lower[lane] = minimum[lane] - margin;
        upper[lane] = maximum[lane] + margin;
    }
    (lower, upper)
}

/// Reference: `rw::collision::FindChildContainingBBox` at
/// `0x82ad6738..0x82ad6820`.
fn find_child_containing_bbox(node_box: AabbWords, bbox: AabbWords) -> Option<u8> {
    let bbox_min = bbox.min_f32();
    let bbox_max = bbox.max_f32();
    let (lower, upper) = overlap_bounds(node_box);
    let mut child = 0_u8;
    for axis in 0..3 {
        let low_delta = bbox_min[axis] - lower[axis];
        let high_delta = upper[axis] - bbox_max[axis];
        if 0.0 >= low_delta && 0.0 >= high_delta {
            return None;
        }
        if low_delta > high_delta {
            child |= 1 << axis;
        }
    }
    Some(child)
}

/// Reference: `rw::collision::BBoxInCorner` at
/// `0x82ad6828..0x82ad68d8`.
fn bbox_in_corner(node_box: AabbWords, bbox: AabbWords) -> bool {
    let bbox_min = bbox.min_f32();
    let bbox_max = bbox.max_f32();
    let (lower, upper) = overlap_bounds(node_box);
    (0..3).all(|axis| upper[axis] > bbox_max[axis] || bbox_min[axis] > lower[axis])
}

fn child_bbox(node_box: AabbWords, child: u8) -> AabbWords {
    let minimum = node_box.min_f32();
    let maximum = node_box.max_f32();
    let (lower, upper) = overlap_bounds(node_box);
    let mut child_min = minimum;
    let mut child_max = upper;
    for axis in 0..3 {
        if child & (1 << axis) != 0 {
            child_min[axis] = lower[axis];
            child_max[axis] = maximum[axis];
        }
    }
    AabbWords::from_f32(child_min, child_max)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn box_f32(min: [f32; 3], max: [f32; 3]) -> AabbWords {
        AabbWords::from_f32(
            [min[0], min[1], min[2], 17.0],
            [max[0], max[1], max[2], 29.0],
        )
    }

    fn root_extent() -> AabbWords {
        box_f32([-10.0; 3], [10.0; 3])
    }

    fn list(tree: &LocalOctree, mut head: PackedLink) -> Vec<usize> {
        let mut result = Vec::new();
        while let Some(index) = head.index() {
            result.push(index);
            head = tree.entries[index].next;
        }
        result
    }

    fn corner_box(offset: f32) -> AabbWords {
        box_f32([-9.0 + offset; 3], [-8.5 + offset; 3])
    }

    fn split_once_tree() -> LocalOctree {
        let mut tree = LocalOctree::new(OctreeCapacity::new(8).unwrap(), root_extent());
        for offset in [0.0, 0.1, 0.2, 0.3] {
            tree.insert_next(corner_box(offset)).unwrap();
        }
        tree
    }

    fn pending_nonroot_collapse() -> LocalOctree {
        let mut tree = split_once_tree();
        for expected in 0..3 {
            assert_eq!(tree.remove_next().unwrap().entry.index(), expected);
        }
        tree
    }

    fn assert_atomic_error(tree: &mut LocalOctree, expected: OctreeRemovalError) {
        let before = tree.clone();
        assert_eq!(tree.remove_next(), Err(expected));
        assert_eq!(*tree, before);
    }

    #[test]
    fn descriptor_and_offsets_match_reference_snapshots() {
        let expected = [
            (0, 1, 0x60, 0x40, 0x60),
            (1, 1, 0x88, 0x60, 0x80),
            (2, 2, 0xd0, 0x80, 0xc0),
            (3, 2, 0xf8, 0xa0, 0xe0),
            (4, 3, 0x140, 0xc0, 0x120),
            (284, 143, 0x3e80, 0x23c0, 0x35a0),
            (285, 143, 0x3ea8, 0x23e0, 0x35c0),
            (65_535, 32_768, 0x38_0018, 0x20_0020, 0x30_0020),
        ];
        for (entries, nodes, total, node_offset, entry_offset) in expected {
            let capacity = OctreeCapacity::new(entries).unwrap();
            let plan = OctreeResourcePlan::for_capacity(capacity).unwrap();
            assert_eq!(plan.node_count, nodes);
            assert_eq!(
                plan.resources[0],
                ResourceSlotPlan {
                    size: total,
                    alignment: 32
                }
            );
            assert_eq!(plan.bbox_offset, 0x40);
            assert_eq!(plan.node_offset, node_offset);
            assert_eq!(plan.entry_offset, entry_offset);
            assert_eq!(
                plan.resources[1..],
                [ResourceSlotPlan {
                    size: 0,
                    alignment: 1
                }; 4]
            );
        }
        assert_eq!(
            OctreeCapacity::new(65_536),
            Err(CapacityError {
                requested: 65_536,
                maximum: 65_535
            })
        );
    }

    #[test]
    fn constructor_normalizes_cube_and_preserves_fourth_lane_center() {
        let input = AabbWords::from_f32([-2.0, -4.0, -1.0, 8.0], [6.0, 2.0, 1.0, 12.0]);
        let tree = LocalOctree::new(OctreeCapacity::new(4).unwrap(), input);
        assert_eq!(
            tree.extent.min_f32().map(f32::to_bits),
            [-2.0, -5.0, -4.0, 10.0].map(f32::to_bits)
        );
        assert_eq!(
            tree.extent.max_f32().map(f32::to_bits),
            [6.0, 3.0, 4.0, 10.0].map(f32::to_bits)
        );
        assert_eq!(tree.max_nodes, 3);
        assert_eq!(tree.node_free_list.raw(), 1);
        assert_eq!(tree.active_node(0).parent.raw(), 0);
        assert_eq!(tree.active_node(0).child_types, 0xff);
        assert!(matches!(tree.nodes[1], NodeSlot::Free { next } if next.raw() == 2));
        assert!(matches!(tree.nodes[2], NodeSlot::Free { next } if next == PackedLink::NONE));
    }

    #[test]
    fn zero_and_one_capacity_preserve_native_root_terminator_edge() {
        for capacity in [0, 1] {
            let tree = LocalOctree::new(OctreeCapacity::new(capacity).unwrap(), root_extent());
            assert_eq!(tree.nodes.len(), 1);
            assert_eq!(tree.active_node(0).parent, PackedLink::NONE);
            assert_eq!(tree.node_free_list.raw(), 1);
        }
    }

    #[test]
    fn all_eight_child_numbers_and_strict_tie_are_stable() {
        let node = box_f32([0.0; 3], [10.0; 3]);
        for child in 0_u8..8 {
            let mut min = [1.0; 3];
            let mut max = [2.0; 3];
            for axis in 0..3 {
                if child & (1 << axis) != 0 {
                    min[axis] = 8.0;
                    max[axis] = 9.0;
                }
            }
            assert_eq!(
                find_child_containing_bbox(node, box_f32(min, max)),
                Some(child)
            );
        }

        // lower=4 and upper=6: [4.5,5.5] has equal deltas and selects low.
        assert_eq!(
            find_child_containing_bbox(node, box_f32([4.5; 3], [5.5; 3])),
            Some(0)
        );
        // Inclusive overlap spanning on any axis produces the stuck sentinel.
        assert_eq!(
            find_child_containing_bbox(node, box_f32([4.0, 1.0, 1.0], [6.0, 2.0, 2.0])),
            None
        );
    }

    #[test]
    fn corner_predicate_uses_strict_overlap_boundaries() {
        let node = box_f32([0.0; 3], [10.0; 3]);
        // lower=4 and upper=6. Equality satisfies neither strict disjunct.
        assert!(!bbox_in_corner(node, box_f32([4.0; 3], [6.0; 3])));

        let below_upper = f32::from_bits(6.0_f32.to_bits() - 1);
        assert!(bbox_in_corner(node, box_f32([4.0; 3], [below_upper; 3])));

        let above_lower = f32::from_bits(4.0_f32.to_bits() + 1);
        assert!(bbox_in_corner(node, box_f32([above_lower; 3], [6.0; 3])));
    }

    #[test]
    fn root_bounds_are_inclusive_and_outside_entries_are_lifo_stuck() {
        let mut tree = LocalOctree::new(OctreeCapacity::new(3).unwrap(), root_extent());
        tree.insert_next(box_f32([-10.0; 3], [10.0; 3])).unwrap();
        tree.insert_next(box_f32([-11.0, 0.0, 0.0], [-10.5, 1.0, 1.0]))
            .unwrap();
        assert_eq!(list(&tree, tree.active_node(0).stuck_entries), vec![1, 0]);
        assert_eq!(tree.entries[0].child, CHILD_SENTINEL);
        assert_eq!(tree.entries[1].group, 0);
        assert_eq!(tree.bboxes[0], box_f32([-10.0; 3], [10.0; 3]));
    }

    #[test]
    fn leaf_entries_are_lifo_and_noncorner_does_not_increment() {
        let mut tree = LocalOctree::new(OctreeCapacity::new(6).unwrap(), root_extent());
        let corner = box_f32([-9.0; 3], [-8.0; 3]);
        let noncorner = box_f32([-9.0, -9.0, -9.0], [1.0, -8.0, -8.0]);
        tree.insert_next(corner).unwrap();
        tree.insert_next(noncorner).unwrap();
        assert_eq!(list(&tree, tree.active_node(0).child_refs[0]), vec![1, 0]);
        assert_eq!(tree.active_node(0).push_counts[0], 1);
        assert_eq!(tree.entries[0].corner, 1);
        assert_eq!(tree.entries[1].corner, 0);
    }

    #[test]
    fn fourth_corner_entry_splits_once_and_redistribution_reverses_list() {
        let mut tree = LocalOctree::new(OctreeCapacity::new(8).unwrap(), root_extent());
        for offset in [0.0, 0.1, 0.2] {
            tree.insert_next(box_f32([-9.0 + offset; 3], [-8.5 + offset; 3]))
                .unwrap();
        }
        assert!(tree.active_node(0).child_is_leaf(0));
        tree.insert_next(box_f32([-8.7; 3], [-8.2; 3])).unwrap();

        assert!(!tree.active_node(0).child_is_leaf(0));
        assert_eq!(tree.active_node(0).child_refs[0].raw(), 1);
        assert_eq!(tree.node_free_list.raw(), 2);
        // The old newest-to-oldest list is pushed into the new leaf, producing
        // the oldest-to-newest sequence.
        assert_eq!(
            list(&tree, tree.active_node(1).child_refs[0]),
            vec![0, 1, 2, 3]
        );
        assert_eq!(tree.active_node(1).push_counts[0], 4);
    }

    #[test]
    fn redistribution_moves_noncorner_entries_to_new_node_stuck_list() {
        let mut tree = LocalOctree::new(OctreeCapacity::new(10).unwrap(), root_extent());
        let noncorner = box_f32([-9.0, -9.0, -9.0], [1.0, -8.0, -8.0]);
        tree.insert_next(noncorner).unwrap();
        for offset in [0.0, 0.1, 0.2, 0.3] {
            tree.insert_next(box_f32([-9.0 + offset; 3], [-8.5 + offset; 3]))
                .unwrap();
        }
        assert_eq!(list(&tree, tree.active_node(1).stuck_entries), vec![0]);
        assert_eq!(tree.entries[0].child, CHILD_SENTINEL);
        assert_eq!(tree.entries[0].node.raw(), 1);
    }

    #[test]
    fn exhausted_node_pool_suppresses_split_without_failing_insert() {
        let mut tree = LocalOctree::new(OctreeCapacity::new(8).unwrap(), root_extent());
        for offset in [0.0, 0.1, 0.2, 0.3] {
            tree.insert_next(box_f32([-9.0 + offset; 3], [-8.5 + offset; 3]))
                .unwrap();
        }
        // Force the exact normal exhaustion state after the first allocation.
        tree.node_free_list = PackedLink::NONE;
        for offset in [0.0, 0.1, 0.2, 0.3] {
            tree.insert_next(box_f32([-4.8 + offset; 3], [-4.4 + offset; 3]))
                .unwrap();
        }
        assert_eq!(tree.entries.len(), 8);
        assert_eq!(tree.node_free_list, PackedLink::NONE);
    }

    #[test]
    fn push_count_saturates() {
        let mut tree = LocalOctree::new(OctreeCapacity::new(1).unwrap(), root_extent());
        tree.active_node_mut(0).push_counts[0] = u8::MAX;
        tree.node_free_list = PackedLink::NONE;
        tree.insert_next(box_f32([-9.0; 3], [-8.0; 3])).unwrap();
        assert_eq!(tree.active_node(0).push_counts[0], u8::MAX);
    }

    #[test]
    fn sequential_capacity_error_leaves_storage_unchanged() {
        let mut tree = LocalOctree::new(OctreeCapacity::new(1).unwrap(), root_extent());
        let first = box_f32([-1.0; 3], [1.0; 3]);
        tree.insert_next(first).unwrap();
        let before = tree.clone();
        assert_eq!(
            tree.insert_next(first),
            Err(OctreeInsertError::Full(OctreeFull { capacity: 1 }))
        );
        assert_eq!(tree, before);
    }

    #[test]
    fn child_bbox_preserves_fourth_lane_low_selection() {
        let node = AabbWords::from_f32([0.0, 0.0, 0.0, 4.0], [10.0, 10.0, 10.0, 8.0]);
        let child = child_bbox(node, 7);
        assert_eq!(child.min_f32()[3].to_bits(), 4.0_f32.to_bits());
        // w uses the low-child upper calculation; the +0 overlap scale keeps max.w.
        assert_eq!(child.max_f32()[3].to_bits(), 8.0_f32.to_bits());
    }

    #[test]
    fn removal_01_empty_prefix_is_an_atomic_error() {
        let mut tree = LocalOctree::new(OctreeCapacity::new(1).unwrap(), root_extent());
        assert_atomic_error(
            &mut tree,
            OctreeRemovalError::NoInsertedEntry {
                next_removal: 0,
                inserted: 0,
            },
        );
    }

    #[test]
    fn removal_02_leaf_head_unlinks_and_preserves_target_storage() {
        let mut tree = split_once_tree();
        let entry_before = tree.entries[0];
        let bbox_before = tree.bboxes[0];
        assert_eq!(
            list(&tree, tree.active_node(1).child_refs[0]),
            vec![0, 1, 2, 3]
        );

        assert_eq!(
            tree.remove_next().unwrap(),
            OctreeRemoval {
                entry: EntryId(0),
                collapsed_node: None,
            }
        );
        assert_eq!(
            list(&tree, tree.active_node(1).child_refs[0]),
            vec![1, 2, 3]
        );
        assert_eq!(tree.entries[0], entry_before);
        assert_eq!(tree.bboxes[0], bbox_before);
    }

    #[test]
    fn removal_03_leaf_middle_unlinks_only_predecessor() {
        let mut tree = split_once_tree();
        tree.node_free_list = PackedLink::NONE;
        tree.insert_next(corner_box(0.4)).unwrap();
        assert_eq!(
            list(&tree, tree.active_node(1).child_refs[0]),
            vec![4, 0, 1, 2, 3]
        );
        let target_before = tree.entries[0];

        tree.remove_next().unwrap();
        assert_eq!(
            list(&tree, tree.active_node(1).child_refs[0]),
            vec![4, 1, 2, 3]
        );
        assert_eq!(tree.entries[0], target_before);
        assert_eq!(tree.entries[4].next.raw(), 1);
    }

    #[test]
    fn removal_04_leaf_tail_unlinks_only_predecessor() {
        let mut tree = LocalOctree::new(OctreeCapacity::new(3).unwrap(), root_extent());
        for offset in [0.0, 0.1, 0.2] {
            tree.insert_next(corner_box(offset)).unwrap();
        }
        assert_eq!(
            list(&tree, tree.active_node(0).child_refs[0]),
            vec![2, 1, 0]
        );

        tree.remove_next().unwrap();
        assert_eq!(list(&tree, tree.active_node(0).child_refs[0]), vec![2, 1]);
        assert_eq!(tree.entries[1].next, PackedLink::NONE);
    }

    #[test]
    fn removal_05_stuck_head_middle_and_tail_use_the_stuck_head() {
        for (links, expected) in [
            ([0_u16, 1, 2], vec![1, 2]),
            ([2_u16, 0, 1], vec![2, 1]),
            ([2_u16, 1, 0], vec![2, 1]),
        ] {
            let mut tree = LocalOctree::new(OctreeCapacity::new(3).unwrap(), root_extent());
            for offset in [0.0, 1.0, 2.0] {
                tree.insert_next(box_f32(
                    [-12.0 - offset, 0.0, 0.0],
                    [-11.0 - offset, 1.0, 1.0],
                ))
                .unwrap();
            }
            tree.active_node_mut(0).stuck_entries = PackedLink(links[0]);
            tree.entries[usize::from(links[0])].next = PackedLink(links[1]);
            tree.entries[usize::from(links[1])].next = PackedLink(links[2]);
            tree.entries[usize::from(links[2])].next = PackedLink::NONE;

            tree.remove_next().unwrap();
            assert_eq!(list(&tree, tree.active_node(0).stuck_entries), expected);
        }
    }

    #[test]
    fn removal_06_noncorner_leaf_does_not_touch_push_count() {
        let mut tree = LocalOctree::new(OctreeCapacity::new(2).unwrap(), root_extent());
        let noncorner = box_f32([-9.0, -9.0, -9.0], [1.0, -8.0, -8.0]);
        tree.insert_next(noncorner).unwrap();
        assert_eq!(tree.entries[0].corner, 0);
        tree.active_node_mut(0).push_counts[0] = 37;

        tree.remove_next().unwrap();
        assert_eq!(tree.active_node(0).push_counts[0], 37);
    }

    #[test]
    fn removal_07_unsaturated_corner_count_decrements() {
        let mut tree = LocalOctree::new(OctreeCapacity::new(3).unwrap(), root_extent());
        for offset in [0.0, 0.1, 0.2] {
            tree.insert_next(corner_box(offset)).unwrap();
        }
        assert_eq!(tree.active_node(0).push_counts[0], 3);
        tree.remove_next().unwrap();
        assert_eq!(tree.active_node(0).push_counts[0], 2);
    }

    #[test]
    fn removal_08_saturated_recount_is_pre_unlink_255_then_254() {
        let mut tree = LocalOctree::new(OctreeCapacity::new(255).unwrap(), root_extent());
        tree.node_free_list = PackedLink::NONE;
        for _ in 0..255 {
            tree.insert_next(corner_box(0.0)).unwrap();
        }
        assert_eq!(tree.active_node(0).push_counts[0], u8::MAX);

        tree.remove_next().unwrap();
        assert_eq!(tree.active_node(0).push_counts[0], u8::MAX);
        tree.remove_next().unwrap();
        assert_eq!(tree.active_node(0).push_counts[0], 254);
        assert_eq!(list(&tree, tree.active_node(0).child_refs[0]).len(), 253);
    }

    #[test]
    fn removal_09_root_never_collapses() {
        let mut tree = LocalOctree::new(OctreeCapacity::new(1).unwrap(), root_extent());
        tree.insert_next(corner_box(0.0)).unwrap();
        let outcome = tree.remove_next().unwrap();
        assert_eq!(outcome.collapsed_node, None);
        assert!(matches!(tree.nodes[0], NodeSlot::Active(_)));
    }

    #[test]
    fn removal_10_empty_nonroot_collapses_onto_free_list_lifo() {
        let mut tree = pending_nonroot_collapse();
        assert_eq!(tree.node_free_list.raw(), 2);

        let outcome = tree.remove_next().unwrap();
        assert_eq!(outcome.collapsed_node, Some(1));
        assert!(tree.active_node(0).child_is_leaf(0));
        assert_eq!(tree.active_node(0).child_refs[0], PackedLink::NONE);
        assert_eq!(tree.active_node(0).push_counts[0], 0);
        assert_eq!(tree.node_free_list.raw(), 1);
        assert!(matches!(tree.nodes[1], NodeSlot::Free { next } if next.raw() == 2));
    }

    #[test]
    fn removal_11_collapse_is_not_recursive() {
        let mut tree = LocalOctree::new(OctreeCapacity::new(4).unwrap(), root_extent());
        for offset in [0.0, 0.1, 0.2, 0.3] {
            tree.insert_next(corner_box(offset)).unwrap();
        }
        let level_one_box = child_bbox(tree.extent, 0);
        tree.split_leaf(1, 0, child_bbox(level_one_box, 0));
        assert!(!tree.active_node(0).child_is_leaf(0));
        assert!(!tree.active_node(1).child_is_leaf(0));
        for _ in 0..3 {
            tree.remove_next().unwrap();
        }

        let outcome = tree.remove_next().unwrap();
        assert_eq!(outcome.collapsed_node, Some(2));
        assert!(!tree.active_node(0).child_is_leaf(0));
        assert_eq!(tree.active_node(0).child_refs[0].raw(), 1);
        assert!(tree.active_node(1).child_is_leaf(0));
        assert_eq!(tree.active_node(1).child_refs[0], PackedLink::NONE);
    }

    #[test]
    fn removal_12_phase_is_monotonic_and_rejects_later_insertion() {
        let mut tree = LocalOctree::new(OctreeCapacity::new(2).unwrap(), root_extent());
        tree.insert_next(corner_box(0.0)).unwrap();
        tree.remove_next().unwrap();
        assert_eq!(tree.next_removal(), 1);
        let before = tree.clone();
        assert_eq!(
            tree.insert_next(corner_box(0.1)),
            Err(OctreeInsertError::RemovalStarted { next_removal: 1 })
        );
        assert_eq!(tree, before);
        assert_atomic_error(
            &mut tree,
            OctreeRemovalError::NoInsertedEntry {
                next_removal: 1,
                inserted: 1,
            },
        );
    }

    #[test]
    fn removal_13_missing_out_of_range_and_free_node_are_atomic() {
        let mut missing = LocalOctree::new(OctreeCapacity::new(1).unwrap(), root_extent());
        missing.insert_next(corner_box(0.0)).unwrap();
        missing.entries[0].node = PackedLink::NONE;
        assert_atomic_error(
            &mut missing,
            OctreeRemovalError::MissingEntryNode { entry: 0 },
        );

        let mut outside = LocalOctree::new(OctreeCapacity::new(1).unwrap(), root_extent());
        outside.insert_next(corner_box(0.0)).unwrap();
        outside.entries[0].node = PackedLink(9);
        assert_atomic_error(
            &mut outside,
            OctreeRemovalError::NodeOutOfRange { entry: 0, node: 9 },
        );

        let mut free = LocalOctree::new(OctreeCapacity::new(4).unwrap(), root_extent());
        free.insert_next(corner_box(0.0)).unwrap();
        free.entries[0].node = PackedLink(1);
        assert_atomic_error(
            &mut free,
            OctreeRemovalError::NodeIsFree { entry: 0, node: 1 },
        );
    }

    #[test]
    fn removal_14_invalid_or_internal_child_is_atomic() {
        let mut outside = LocalOctree::new(OctreeCapacity::new(1).unwrap(), root_extent());
        outside.insert_next(corner_box(0.0)).unwrap();
        outside.entries[0].child = 8;
        assert_atomic_error(
            &mut outside,
            OctreeRemovalError::ChildOutOfRange { entry: 0, child: 8 },
        );

        let mut internal = split_once_tree();
        internal.active_node_mut(1).child_types &= !1;
        assert_atomic_error(
            &mut internal,
            OctreeRemovalError::EntryInInternalChild {
                entry: 0,
                node: 1,
                child: 0,
            },
        );
    }

    #[test]
    fn removal_15_missing_and_out_of_range_list_links_are_atomic() {
        let mut missing = LocalOctree::new(OctreeCapacity::new(1).unwrap(), root_extent());
        missing.insert_next(corner_box(0.0)).unwrap();
        missing.active_node_mut(0).child_refs[0] = PackedLink::NONE;
        assert_atomic_error(
            &mut missing,
            OctreeRemovalError::TargetNotInSelectedList { entry: 0 },
        );

        let mut outside = LocalOctree::new(OctreeCapacity::new(1).unwrap(), root_extent());
        outside.insert_next(corner_box(0.0)).unwrap();
        outside.active_node_mut(0).child_refs[0] = PackedLink(9);
        assert_atomic_error(
            &mut outside,
            OctreeRemovalError::ListLinkOutOfRange { entry: 0, link: 9 },
        );
    }

    #[test]
    fn removal_16_list_cycle_is_detected_after_target_without_writes() {
        let mut tree = LocalOctree::new(OctreeCapacity::new(2).unwrap(), root_extent());
        tree.insert_next(corner_box(0.0)).unwrap();
        tree.insert_next(corner_box(0.1)).unwrap();
        tree.entries[0].next = PackedLink(1);
        assert_atomic_error(&mut tree, OctreeRemovalError::ListCycle { entry: 0, at: 1 });
    }

    #[test]
    fn removal_17_membership_and_removed_entry_links_are_atomic() {
        let mut mismatch = LocalOctree::new(OctreeCapacity::new(2).unwrap(), root_extent());
        mismatch.insert_next(corner_box(0.0)).unwrap();
        mismatch.insert_next(corner_box(0.1)).unwrap();
        mismatch.entries[1].child = 1;
        assert_atomic_error(
            &mut mismatch,
            OctreeRemovalError::ListMembershipMismatch {
                entry: 0,
                listed: 1,
            },
        );

        let mut removed = LocalOctree::new(OctreeCapacity::new(2).unwrap(), root_extent());
        removed.insert_next(corner_box(0.0)).unwrap();
        removed.insert_next(corner_box(0.1)).unwrap();
        removed.remove_next().unwrap();
        removed.entries[1].next = PackedLink(0);
        assert_atomic_error(
            &mut removed,
            OctreeRemovalError::ListContainsRemovedEntry {
                entry: 1,
                listed: 0,
            },
        );
    }

    #[test]
    fn removal_18_corner_count_underflow_is_atomic() {
        let mut tree = LocalOctree::new(OctreeCapacity::new(1).unwrap(), root_extent());
        tree.insert_next(corner_box(0.0)).unwrap();
        tree.active_node_mut(0).push_counts[0] = 0;
        assert_atomic_error(
            &mut tree,
            OctreeRemovalError::PushCountUnderflow {
                entry: 0,
                node: 0,
                child: 0,
            },
        );
    }

    #[test]
    fn removal_19_all_collapse_link_failures_are_atomic() {
        let mut missing = pending_nonroot_collapse();
        missing.active_node_mut(1).parent = PackedLink::NONE;
        assert_atomic_error(&mut missing, OctreeRemovalError::MissingParent { node: 1 });

        let mut outside = pending_nonroot_collapse();
        outside.active_node_mut(1).parent = PackedLink(9);
        assert_atomic_error(
            &mut outside,
            OctreeRemovalError::ParentOutOfRange { node: 1, parent: 9 },
        );

        let mut free_parent = pending_nonroot_collapse();
        free_parent.active_node_mut(1).parent = PackedLink(2);
        assert_atomic_error(
            &mut free_parent,
            OctreeRemovalError::ParentIsFree { node: 1, parent: 2 },
        );

        let mut bad_child = pending_nonroot_collapse();
        bad_child.active_node_mut(1).child_of_parent = 8;
        assert_atomic_error(
            &mut bad_child,
            OctreeRemovalError::ParentChildOutOfRange { node: 1, child: 8 },
        );

        let mut mismatch = pending_nonroot_collapse();
        mismatch.active_node_mut(0).child_refs[0] = PackedLink::NONE;
        assert_atomic_error(
            &mut mismatch,
            OctreeRemovalError::ParentLinkMismatch {
                node: 1,
                parent: 0,
                child: 0,
            },
        );

        let mut free_outside = pending_nonroot_collapse();
        free_outside.node_free_list = PackedLink(9);
        assert_atomic_error(
            &mut free_outside,
            OctreeRemovalError::FreeListOutOfRange { head: 9 },
        );

        let mut free_active = pending_nonroot_collapse();
        free_active.node_free_list = PackedLink(0);
        assert_atomic_error(
            &mut free_active,
            OctreeRemovalError::FreeListHeadIsActive { head: 0 },
        );
    }

    #[test]
    fn removal_20_saturated_recount_caps_with_more_than_255_live_corners() {
        let mut tree = LocalOctree::new(OctreeCapacity::new(300).unwrap(), root_extent());
        tree.node_free_list = PackedLink::NONE;
        for _ in 0..300 {
            tree.insert_next(corner_box(0.0)).unwrap();
        }
        assert_eq!(tree.active_node(0).push_counts[0], u8::MAX);

        tree.remove_next().unwrap();
        assert_eq!(tree.active_node(0).push_counts[0], u8::MAX);
        assert_eq!(list(&tree, tree.active_node(0).child_refs[0]).len(), 299);
    }

    #[test]
    fn removal_21_nonempty_sibling_leaf_prevents_collapse() {
        let mut tree = LocalOctree::new(OctreeCapacity::new(5).unwrap(), root_extent());
        for offset in [0.0, 0.1, 0.2, 0.3] {
            tree.insert_next(corner_box(offset)).unwrap();
        }
        tree.insert_next(box_f32([-3.5; 3], [-3.0; 3])).unwrap();

        for _ in 0..3 {
            tree.remove_next().unwrap();
        }
        let outcome = tree.remove_next().unwrap();

        assert_eq!(outcome.collapsed_node, None);
        assert_eq!(
            list(&tree, tree.active_node(1).child_refs[0]),
            Vec::<usize>::new()
        );
        assert_eq!(list(&tree, tree.active_node(1).child_refs[7]), vec![4]);
        assert!(matches!(tree.nodes[1], NodeSlot::Active(_)));
    }
}
