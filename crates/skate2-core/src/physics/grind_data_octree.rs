//! Retained `RenderWare` octree build82adbf58/6af0 and box query824d6af8.
#![expect(
    clippy::manual_midpoint,
    reason = "source rounds the sum before halving, including native overflow behavior"
)]
//! Source loose-child masks and ordered entry chains; no generic tree substitute.
use std::fmt;

const NONE: u16 = u16::MAX;
const QUERY_STACK_CAPACITY: usize = 211;
// Static initializer82f7a710: XYZ -0.4, W +0. It is not a tuning cvar.
const INBOX: [f32; 4] = [
    f32::from_bits(0xbecc_cccd),
    f32::from_bits(0xbecc_cccd),
    f32::from_bits(0xbecc_cccd),
    0.0,
];
// Static initializers82f7a748/780/7b8/7f0/820 supply axis bits1,2,4,
// low AA/CC/F0, high55/33/0F and eight bit-selected child masks (W zero).
const LOW_MASKS: [u8; 3] = [0xaa, 0xcc, 0xf0];
const HIGH_MASKS: [u8; 3] = [0x55, 0x33, 0x0f];

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct OctreeBounds {
    pub(crate) minimum: [f32; 4],
    pub(crate) maximum: [f32; 4],
}
impl OctreeBounds {
    /// Native separating-axis comparisons ignore W and reject only ordered >.
    pub(crate) fn overlaps(self, other: Self) -> bool {
        !(0..3).any(|axis| {
            self.minimum[axis] > other.maximum[axis] || other.minimum[axis] > self.maximum[axis]
        })
    }
    fn contains(self, other: Self) -> bool {
        !(0..3).any(|axis| {
            self.minimum[axis] > other.minimum[axis] || other.maximum[axis] > self.maximum[axis]
        })
    }
    fn split_planes(self) -> ([f32; 4], [f32; 4]) {
        let offset = std::array::from_fn::<_, 4, _>(|axis| {
            (self.maximum[axis] - self.minimum[axis]) * INBOX[axis]
        });
        (
            std::array::from_fn(|axis| self.minimum[axis] - offset[axis]),
            std::array::from_fn(|axis| self.maximum[axis] + offset[axis]),
        )
    }
    fn child(self, index: usize, high_min: [f32; 4], low_max: [f32; 4]) -> Self {
        Self {
            minimum: std::array::from_fn(|axis| {
                if axis < 3 && index & (1 << axis) != 0 {
                    high_min[axis]
                } else {
                    self.minimum[axis]
                }
            }),
            maximum: std::array::from_fn(|axis| {
                if axis < 3 && index & (1 << axis) != 0 {
                    self.maximum[axis]
                } else {
                    low_max[axis]
                }
            }),
        }
    }
    /// `FindChildContainingBBox82ad6738` retains the two subtractions before its
    /// comparisons: algebraic midpoint replacement changes rounding and NaNs.
    fn containing_child(self, bbox: Self) -> Option<(usize, Self)> {
        let (high_min, low_max) = self.split_planes();
        let lower = std::array::from_fn::<_, 4, _>(|axis| bbox.minimum[axis] - high_min[axis]);
        let upper = std::array::from_fn::<_, 4, _>(|axis| low_max[axis] - bbox.maximum[axis]);
        if (0..3).any(|axis| 0.0 >= lower[axis] && 0.0 >= upper[axis]) {
            return None;
        }
        let mut index = 0;
        for axis in 0..3 {
            if lower[axis] > upper[axis] {
                index |= 1 << axis;
            }
        }
        Some((index, self.child(index, high_min, low_max)))
    }
    fn in_corner(self, bbox: Self) -> bool {
        let (high_min, low_max) = self.split_planes();
        (0..3).all(|axis| low_max[axis] > bbox.maximum[axis] || bbox.minimum[axis] > high_min[axis])
    }
    fn intersected_children(self, bbox: Self) -> (u8, [f32; 4], [f32; 4]) {
        let (high_min, low_max) = self.split_planes();
        let mut mask = 0xff;
        for axis in 0..3 {
            if bbox.minimum[axis] > low_max[axis] {
                mask &= LOW_MASKS[axis];
            }
            if high_min[axis] > bbox.maximum[axis] {
                mask &= HIGH_MASKS[axis];
            }
        }
        (mask, high_min, low_max)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum GrindOctreeError {
    EntryCapacity(u32),
    Allocation,
    EntryIndex(usize),
    DuplicateEntry(usize),
    UninitializedEntry(usize),
    NodeIndex(usize),
    SplitWithoutContainingChild(usize),
    QueryStackCapacity,
}
impl fmt::Display for GrindOctreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EntryCapacity(n) => write!(
                f,
                "octree capacity {n} exceeds usable native u16 entry indexes"
            ),
            Self::Allocation => f.write_str("octree host storage allocation failed"),
            Self::EntryIndex(n) => write!(f, "octree entry index {n} exceeds source allocation"),
            Self::DuplicateEntry(n) => write!(f, "octree entry {n} is already inserted"),
            Self::UninitializedEntry(n) => {
                write!(f, "octree entry {n} has no initialized source payload")
            }
            Self::NodeIndex(n) => write!(f, "octree node {n} is unavailable"),
            Self::SplitWithoutContainingChild(n) => {
                write!(f, "octree corner entry {n} has no child during split")
            }
            Self::QueryStackCapacity => {
                f.write_str("octree query exceeds its native 211 stack entries")
            }
        }
    }
}
#[derive(Clone, Copy)]
struct Entry {
    next: u16,
    corner: bool,
}
struct Node {
    stuck: u16,
    children: [u16; 8],
    leaf_mask: u8,
    push_counts: [u8; 8],
}
impl Node {
    fn new() -> Self {
        Self {
            stuck: NONE,
            children: [NONE; 8],
            leaf_mask: 0xff,
            push_counts: [0; 8],
        }
    }
}
struct NodeSlot {
    free_next: u16,
    node: Option<Node>,
}
pub(crate) struct GrindDataOctree {
    extent: OctreeBounds,
    boxes: Vec<Option<OctreeBounds>>,
    entries: Vec<Option<Entry>>,
    nodes: Vec<NodeSlot>,
    free_node: u16,
}
impl GrindDataOctree {
    pub(crate) fn try_new(
        max_entries: u32,
        extent: OctreeBounds,
    ) -> Result<Self, GrindOctreeError> {
        if max_entries > u32::from(NONE) {
            return Err(GrindOctreeError::EntryCapacity(max_entries));
        }
        let count = usize::try_from(max_entries).expect("u16 capacity fits usize");
        let node_count = (count >> 1) + 1;
        let mut boxes = Vec::new();
        let mut entries = Vec::new();
        let mut nodes = Vec::new();
        boxes
            .try_reserve_exact(count)
            .map_err(|_| GrindOctreeError::Allocation)?;
        entries
            .try_reserve_exact(count)
            .map_err(|_| GrindOctreeError::Allocation)?;
        nodes
            .try_reserve_exact(node_count)
            .map_err(|_| GrindOctreeError::Allocation)?;
        boxes.resize(count, None);
        entries.resize(count, None);
        // Constructor rounds to a cube using fsubs/fsel twice, not Float::max.
        let half = std::array::from_fn::<_, 4, _>(|axis| {
            (extent.maximum[axis] - extent.minimum[axis]) * 0.5
        });
        let center = std::array::from_fn::<_, 4, _>(|axis| {
            (extent.maximum[axis] + extent.minimum[axis]) * 0.5
        });
        let xy = if half[0] - half[1] >= 0.0 {
            half[0]
        } else {
            half[1]
        };
        let radius = if xy - half[2] >= 0.0 { xy } else { half[2] };
        let half_cube = [radius, radius, radius, 0.0];
        let extent = OctreeBounds {
            minimum: std::array::from_fn(|axis| center[axis] - half_cube[axis]),
            maximum: std::array::from_fn(|axis| center[axis] + half_cube[axis]),
        };
        for index in 0..node_count {
            nodes.push(NodeSlot {
                free_next: if index + 1 == node_count {
                    NONE
                } else {
                    u16::try_from(index + 1).expect("bounded node count")
                },
                node: None,
            });
        }
        nodes[0].node = Some(Node::new());
        // Native stores1 even for the degenerate one-node allocation. A later
        // attempted split refuses at that unavailable node instead of fixing it.
        Ok(Self {
            extent,
            boxes,
            entries,
            nodes,
            free_node: 1,
        })
    }
    pub(crate) fn try_insert(
        &mut self,
        index: usize,
        bbox: OctreeBounds,
    ) -> Result<(), GrindOctreeError> {
        let destination = self
            .boxes
            .get_mut(index)
            .ok_or(GrindOctreeError::EntryIndex(index))?;
        if destination.is_some() {
            return Err(GrindOctreeError::DuplicateEntry(index));
        }
        *destination = Some(bbox);
        let mut node_index = 0;
        let mut node_box = self.extent;
        let mut selected = if node_box.contains(bbox) {
            node_box.containing_child(bbox)
        } else {
            None
        };
        loop {
            let Some((child, child_box)) = selected else {
                let head = self.node(node_index)?.stuck;
                self.entries[index] = Some(Entry {
                    next: head,
                    corner: false,
                });
                self.node_mut(node_index)?.stuck = u16::try_from(index).expect("bounded entry");
                return Ok(());
            };
            node_box = child_box;
            if self.node(node_index)?.leaf_mask & (1 << child) != 0 {
                self.add_leaf(index, node_index, child, node_box.in_corner(bbox))?;
                if self.node(node_index)?.push_counts[child] > 3 && self.free_node != NONE {
                    self.split_leaf(node_index, child, node_box)?;
                }
                return Ok(());
            }
            node_index = usize::from(self.node(node_index)?.children[child]);
            selected = node_box.containing_child(bbox);
        }
    }
    fn node(&self, index: usize) -> Result<&Node, GrindOctreeError> {
        self.nodes
            .get(index)
            .and_then(|slot| slot.node.as_ref())
            .ok_or(GrindOctreeError::NodeIndex(index))
    }
    /// Octree::Remove82acc210 and Node::DecPushCount82acc120. Locate the
    /// owning chain in host storage (native entries carry node/child indexes),
    /// unlink in place and free only that empty node, without rebuilding or
    /// changing the query order of surviving entries.
    pub(crate) fn try_remove(&mut self, index: usize) -> Result<(), GrindOctreeError> {
        let entry = self
            .entries
            .get(index)
            .copied()
            .flatten()
            .ok_or(GrindOctreeError::UninitializedEntry(index))?;
        let mut owner = None;
        'nodes: for (node_index, slot) in self.nodes.iter().enumerate() {
            let Some(node) = &slot.node else {
                continue;
            };
            for child in 0..=8 {
                if child < 8 && node.leaf_mask & (1 << child) == 0 {
                    continue;
                }
                let mut current = if child == 8 {
                    node.stuck
                } else {
                    node.children[child]
                };
                let mut previous = None;
                while current != NONE {
                    if usize::from(current) == index {
                        owner = Some((node_index, child, previous));
                        break 'nodes;
                    }
                    previous = Some(usize::from(current));
                    current = self.entries[usize::from(current)]
                        .ok_or(GrindOctreeError::UninitializedEntry(usize::from(current)))?
                        .next;
                }
            }
        }
        let (node_index, child, previous) =
            owner.ok_or(GrindOctreeError::UninitializedEntry(index))?;
        if child < 8 && entry.corner {
            let count = self.node(node_index)?.push_counts[child];
            let count = if count == u8::MAX {
                let mut count = 0_u8;
                let mut current = self.node(node_index)?.children[child];
                while current != NONE && count < u8::MAX {
                    let value = self.entries[usize::from(current)]
                        .ok_or(GrindOctreeError::UninitializedEntry(usize::from(current)))?;
                    count += u8::from(value.corner);
                    current = value.next;
                }
                count
            } else {
                count.wrapping_sub(1)
            };
            self.node_mut(node_index)?.push_counts[child] = count;
        }
        if let Some(previous) = previous {
            self.entries[previous].as_mut().expect("chain entry").next = entry.next;
        } else if child == 8 {
            self.node_mut(node_index)?.stuck = entry.next;
        } else {
            self.node_mut(node_index)?.children[child] = entry.next;
        }
        self.entries[index] = None;
        self.boxes[index] = None;
        let node = self.node(node_index)?;
        if node_index != 0
            && node.stuck == NONE
            && node.leaf_mask == 0xff
            && node.children.iter().all(|head| *head == NONE)
        {
            let parent = self
                .nodes
                .iter()
                .enumerate()
                .find_map(|(i, slot)| {
                    let node = slot.node.as_ref()?;
                    (0..8)
                        .find(|child| {
                            node.leaf_mask & (1 << child) == 0
                                && usize::from(node.children[*child]) == node_index
                        })
                        .map(|child| (i, child))
                })
                .ok_or(GrindOctreeError::NodeIndex(node_index))?;
            let node = self.node_mut(parent.0)?;
            node.children[parent.1] = NONE;
            node.push_counts[parent.1] = 0;
            node.leaf_mask |= 1 << parent.1;
            self.nodes[node_index].node = None;
            self.nodes[node_index].free_next = self.free_node;
            self.free_node = node_index as u16;
        }
        Ok(())
    }
    fn node_mut(&mut self, index: usize) -> Result<&mut Node, GrindOctreeError> {
        self.nodes
            .get_mut(index)
            .and_then(|slot| slot.node.as_mut())
            .ok_or(GrindOctreeError::NodeIndex(index))
    }
    fn add_leaf(
        &mut self,
        index: usize,
        node: usize,
        child: usize,
        corner: bool,
    ) -> Result<(), GrindOctreeError> {
        let head = self.node(node)?.children[child];
        self.entries[index] = Some(Entry { next: head, corner });
        let target = self.node_mut(node)?;
        target.children[child] = u16::try_from(index).expect("bounded entry");
        if corner {
            target.push_counts[child] = target.push_counts[child].saturating_add(1);
        }
        Ok(())
    }
    fn split_leaf(
        &mut self,
        parent: usize,
        child: usize,
        bbox: OctreeBounds,
    ) -> Result<(), GrindOctreeError> {
        let next_node = self.free_node;
        let node_index = usize::from(next_node);
        let mut entry = self.node(parent)?.children[child];
        let slot = self
            .nodes
            .get_mut(node_index)
            .ok_or(GrindOctreeError::NodeIndex(node_index))?;
        self.free_node = slot.free_next;
        slot.node = Some(Node::new());
        let parent = self.node_mut(parent)?;
        parent.children[child] = next_node;
        parent.leaf_mask &= !(1 << child);
        while entry != NONE {
            let index = usize::from(entry);
            let current = self.entries[index].ok_or(GrindOctreeError::UninitializedEntry(index))?;
            if current.corner {
                let entry_box =
                    self.boxes[index].ok_or(GrindOctreeError::UninitializedEntry(index))?;
                let (child, child_box) = bbox
                    .containing_child(entry_box)
                    .ok_or(GrindOctreeError::SplitWithoutContainingChild(index))?;
                self.add_leaf(index, node_index, child, child_box.in_corner(entry_box))?;
            } else {
                let head = self.node(node_index)?.stuck;
                self.entries[index] = Some(Entry {
                    next: head,
                    corner: false,
                });
                self.node_mut(node_index)?.stuck = entry;
            }
            entry = current.next;
        }
        Ok(())
    }
    pub(crate) fn try_query(
        &self,
        bbox: OctreeBounds,
    ) -> Result<GrindOctreeQuery<'_>, GrindOctreeError> {
        let mut stack = Vec::new();
        stack
            .try_reserve_exact(QUERY_STACK_CAPACITY)
            .map_err(|_| GrindOctreeError::Allocation)?;
        stack.push((0, self.extent));
        Ok(GrindOctreeQuery {
            tree: self,
            bbox,
            stack,
            results: Vec::with_capacity(9),
            next_entry: NONE,
        })
    }
}
pub(crate) struct GrindOctreeQuery<'a> {
    tree: &'a GrindDataOctree,
    bbox: OctreeBounds,
    stack: Vec<(usize, OctreeBounds)>,
    results: Vec<u16>,
    next_entry: u16,
}
impl GrindOctreeQuery<'_> {
    pub(crate) fn try_next(&mut self) -> Result<Option<usize>, GrindOctreeError> {
        loop {
            let Some(index) = self.next_intersected()? else {
                return Ok(None);
            };
            let bbox = self.tree.boxes[index].ok_or(GrindOctreeError::UninitializedEntry(index))?;
            if self.bbox.overlaps(bbox) {
                return Ok(Some(index));
            }
        }
    }
    fn next_intersected(&mut self) -> Result<Option<usize>, GrindOctreeError> {
        loop {
            if self.next_entry != NONE {
                let index = usize::from(self.next_entry);
                self.next_entry = self.tree.entries[index]
                    .ok_or(GrindOctreeError::UninitializedEntry(index))?
                    .next;
                return Ok(Some(index));
            }
            if let Some(head) = self.results.pop() {
                self.next_entry = head;
                continue;
            }
            let Some((node_index, bbox)) = self.stack.pop() else {
                return Ok(None);
            };
            let node = self.tree.node(node_index)?;
            if node.stuck != NONE {
                self.results.push(node.stuck);
            }
            let (mask, high_min, low_max) = bbox.intersected_children(self.bbox);
            for child in 0..8 {
                if mask & (1 << child) == 0 {
                    continue;
                }
                let reference = node.children[child];
                if node.leaf_mask & (1 << child) != 0 {
                    if reference != NONE {
                        self.results.push(reference);
                    }
                } else {
                    if self.stack.len() == QUERY_STACK_CAPACITY {
                        return Err(GrindOctreeError::QueryStackCapacity);
                    }
                    self.stack
                        .push((usize::from(reference), bbox.child(child, high_min, low_max)));
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "grind_data_octree_tests.rs"]
mod tests;
