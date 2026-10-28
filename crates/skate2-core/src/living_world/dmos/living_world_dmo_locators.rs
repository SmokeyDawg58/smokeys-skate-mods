//! `Sk8::LW::LWDynamicObjectLocatorMan`: streamed DMO locators, their
//! persistence rows and the passive XZ KD tree searched by the census.
//!
//! Constructor 0x82d3ac40 and the `LWDynamicObjectMan::Init` subsequence
//! 0x82d3e0cc..e124. Locator blocks come from `LWDynamicObjectMemoryMan` store 3
//! (1024 x 0xa0, 0x82d29dcc..dde0); KD nodes from the tree's own "Entity KDTree
//! MemStore" (1024 x 0x50). `MemStore::RequestBlock` 0x82a6e450 returns null
//! once every block is out.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "locator readers without a census caller stay available to tests"
    )
)]
use crate::{
    reckoning_numeric::length_from_squared_and_estimate,
    simulation_drive_jacobian::drive_basis_host, world::try_simulation_quaternion_candidate,
};
use skate2_formats::pegasus::Dmo;
use std::collections::{HashMap, HashSet};

const LOCATOR_BLOCKS: usize = 1024;
const KD_NODE_BLOCKS: usize = 1024;
const PERSISTENT_LOCATORS: usize = 512;
/// `UpdateObjectPersistentData` 0x82d3c6c4: persistence priority 4 always
/// evicts the queue top once the map is full.
const PERSISTENCE_PRIORITY_EVICT: i32 = 4;
/// `fixed_vector<KDNode*, 256>` capacity bounds every node search.
const KD_SEARCH_CAPACITY: usize = 256;
/// `GetPassiveLocatorsWithinRange` 0x82d3ad20 clamps its request to 0x400.
const PASSIVE_QUERY_CLAMP: u32 = 0x400;
/// `KDTree::Remove` 0x82d3a1ec..a210: FindNear(0.1, 4 neighbours).
const REMOVE_RADIUS: f32 = f32::from_bits(0x3dcc_cccd);
const REMOVE_NEIGHBOURS: usize = 4;
/// `EPSILON_160` 0x82327080.
const REMOVE_EPSILON: f32 = f32::from_bits(0x3400_0000);
const SIGN_MASK: u32 = 0x8000_0000;

/// `kEnableTheHackToRemoveBadLocators` rejects these full IDs in `CreateLocator`
/// 0x82d3ad80..ae08.
const BAD_LOCATOR_IDS: [u64; 5] = [
    0xb2ae_dd58_9011_3520,
    0xfdac_a9fa_3097_3a71,
    0xecd1_3b5a_fc3c_cc8c,
    0xda35_14fa_fb4a_1c02,
    0xa9cc_0445_c534_e0d0,
];

/// `DynamicObjectLocator` +0x98 bitfield.
pub(crate) const LOCATOR_STATIC: u8 = 0x08;
pub(crate) const LOCATOR_IN_SESSION_MARKER: u8 = 0x10;
pub(crate) const LOCATOR_PRIORITY_TRACKING: u8 = 0x20;
pub(crate) const LOCATOR_PERSISTENCE: u8 = 0x40;
pub(crate) const LOCATOR_ENABLED: u8 = 0x80;
/// New block, 0x82d3af7c..afa0: `(old & 7) | 0xe0`. The three low bits are the
/// unnamed pad field; a fresh host block holds zero there.
const NEW_LOCATOR_FLAGS: u8 = 0xe0;
/// 0x82d3c064..c08c keeps enabled, in-session marker and the pad bits.
const CHUNK_FLAG_KEEP: u8 = 0x97;

/// `rw::math::vpu::QuatPos`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct QuatPos {
    pub(crate) quat: [f32; 4],
    pub(crate) pos: [f32; 4],
}

/// `Sk8::LW::tLocatorData`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LocatorData {
    pub(crate) transform: QuatPos,
    pub(crate) initial_stream_transform: QuatPos,
    pub(crate) locator_id: u64,
    pub(crate) model_id: u64,
    pub(crate) attachment_time: u32,
}

/// `Sk8::LW::tPersistentLocatorData`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PersistentLocatorData {
    pub(crate) basic: LocatorData,
    pub(crate) priority: i32,
    pub(crate) safety: i32,
}

/// `Sk8::LW::DynamicObjectLocator` (0xa0 bytes).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DynamicObjectLocator {
    pub(crate) transform: [f32; 16],
    pub(crate) initial_stream_transform: [f32; 16],
    pub(crate) locator_id: u64,
    pub(crate) model_id: u64,
    pub(crate) attachment_time: u32,
    pub(crate) user_object_id: u32,
    pub(crate) flags: u8,
}

/// Native `FailAssert` site whose matrix or quaternion lanes held a NaN.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LocatorAssert {
    /// `tLocatorData::SetTransform` 0x82d29720, line 27. The chunk's
    /// `SetInitialStreamTransform` (line 30) checks the same record matrix.
    DataTransform,
    /// `tLocatorData::SetInitialStreamTransform` 0x82d29a40, line 30, reached
    /// by `UpdateObjectPersistentData` with the locator's own matrix.
    DataInitialStreamTransform,
    /// `DynamicObjectLocator::SetTransform` 0x82d29520, line 21.
    LocatorTransform,
    /// `AttachObjectToLocator` line 747: missing or already attached locator.
    AttachUser,
    /// `DestroyLocator` line 461: the locator still has a user object.
    DestroyUser,
    /// `UpdateObjectPersistentData` line 862: the object's locator is missing.
    PersistenceLocator,
    /// `RemoveLocatorFromPersistence` lines 977/987: queue or locator missing.
    PersistenceQueue,
    /// `CreateLocator` line 388: transform position.
    CreatePosition,
    /// `CreateLocator` line 389: transform quaternion, all four lanes.
    CreateQuaternion,
    /// `CreateLocator` line 390: `GetTransform` rows.
    CreateTransform,
    /// `DynamicObjectLocator::SetInitialStreamTransform` 0x82d29608, line 24.
    LocatorInitialStreamTransform,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum LocatorError<E> {
    Services(E),
    CullCallback(String),
    Assert {
        locator_id: u64,
        site: LocatorAssert,
    },
    /// `AddLocatorsForStreamedLocatorChunk` line 209: no registered data.
    MissingChunk {
        asset_id: u64,
    },
    /// kdtree.h line 723: more than 256 requested items.
    FindNearLimit {
        requested: u32,
    },
    /// Unordered distances would take the multiset's NaN route, not reproduced.
    UnorderedDistance,
    Allocation,
}

impl<E: std::fmt::Display> std::fmt::Display for LocatorError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CullCallback(error)=>write!(f,"DMO cull callback: {error}"),
            Self::Services(error) => write!(f, "DMO locator services: {error}"),
            Self::Assert { locator_id, site } => {
                write!(f, "DMO locator {locator_id:016x} failed {site:?}")
            }
            Self::MissingChunk { asset_id } => {
                write!(f, "DMO locator chunk {asset_id:016x} is not registered")
            }
            Self::FindNearLimit { requested } => {
                write!(f, "DMO locator search requested {requested} > 256 items")
            }
            Self::UnorderedDistance => f.write_str("DMO locator search met a NaN distance"),
            Self::Allocation => f.write_str("DMO locator storage allocation failed"),
        }
    }
}

impl<E: std::fmt::Debug + std::fmt::Display> std::error::Error for LocatorError<E> {}

/// Live collaborators read by the native routines.
pub(crate) trait LocatorServices {
    type Error;
    /// `kEnableDMOLocatorLoading`, sampled by `AddStreamedLocatorData` 0x82d3d570.
    fn enable_locator_loading(&mut self) -> Result<bool, Self::Error>;
    /// `kEnableTheHackToRemoveBadLocators`, sampled by each `CreateLocator`.
    fn remove_bad_locators(&mut self) -> Result<bool, Self::Error>;
    /// `GetRegAsset(mSimAssetRegistry, id)`: the kind-31 `tDMOData` records.
    fn locator_chunk(&mut self, asset_id: u64) -> Result<Option<Vec<Dmo>>, Self::Error>;
    /// `vrsqrtefp128` seed.
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error>;
}

fn xyz_valid(row: [f32; 4]) -> bool {
    !(row[0].is_nan() || row[1].is_nan() || row[2].is_nan())
}

fn matrix_valid(matrix: &[f32; 16]) -> bool {
    (0..4).all(|row| {
        xyz_valid([
            matrix[row * 4],
            matrix[row * 4 + 1],
            matrix[row * 4 + 2],
            matrix[row * 4 + 3],
        ])
    })
}

/// `QuatFromMat` in `tLocatorData::SetTransform` 0x82d297d4..9980: the shared
/// unnormalized candidate (sign vectors from the dynamic initializers
/// 0x82f7eec8/ef28/ef58) and the row-3 copy.
fn quat_pos_from_matrix<S: LocatorServices>(
    matrix: &[f32; 16],
    services: &mut S,
) -> Result<QuatPos, S::Error> {
    let quat =
        try_simulation_quaternion_candidate(*matrix, |magnitudes| -> Result<[f32; 4], S::Error> {
            let mut seeds = [0.0; 4];
            for (seed, magnitude) in seeds.iter_mut().zip(magnitudes) {
                *seed = services.reciprocal_sqrt_estimate(magnitude)?;
            }
            Ok(seeds)
        })?;
    Ok(QuatPos {
        quat,
        pos: [matrix[12], matrix[13], matrix[14], matrix[15]],
    })
}

/// `tLocatorData::SetTransform` 0x82d29720 on an object matrix: the line-27
/// NaN assert, then `QuatFromMat`.
fn data_transform<S: LocatorServices>(
    matrix: &[f32; 16],
    locator_id: u64,
    services: &mut S,
) -> Result<QuatPos, LocatorError<S::Error>> {
    if !matrix_valid(matrix) {
        return Err(LocatorError::Assert {
            locator_id,
            site: LocatorAssert::DataTransform,
        });
    }
    quat_pos_from_matrix(matrix, services).map_err(LocatorError::Services)
}

/// `tLocatorData::GetTransform` 0x82d29990 (and the initial-stream twin
/// 0x82d29cb0): the `gSqrt2s` expansion with its noncanonical axis W lanes.
fn matrix_from_quat_pos(value: &QuatPos) -> [f32; 16] {
    let [x, y, z] = drive_basis_host(value.quat);
    let mut matrix = [0.0; 16];
    matrix[0..4].copy_from_slice(&x);
    matrix[4..8].copy_from_slice(&y);
    matrix[8..12].copy_from_slice(&z);
    matrix[12..16].copy_from_slice(&value.pos);
    matrix
}

fn xz(matrix: &[f32; 16]) -> [f32; 2] {
    // VectorPermuteConstant<0,6,0,0> of the w axis.
    [matrix[12], matrix[14]]
}

fn abs_bits(value: f32) -> f32 {
    f32::from_bits(value.to_bits() & !SIGN_MASK)
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct KdNode {
    left: Option<u16>,
    right: Option<u16>,
    parent: Option<u16>,
    /// KDObject `mObject`, node +0x30.
    id: u64,
    /// KDObject `mCoord`, node +0x40.
    coord: [f32; 2],
}

/// `KDTree<cAssetID, Vector2, 2>` with its node `MemStore`. The entity trees
/// (`KDTree<uint, Vector2, 2>`) share `KDTree::Insert` 0x82710e28's graph,
/// `KDNode::Insert` 0x82d2d3f0 and `KDNode::FindNear`, with a smaller store.
pub(crate) struct LocatorKdTree {
    nodes: Vec<Option<KdNode>>,
    free: Vec<u16>,
    root: Option<u16>,
}

/// PC Object Dropper locator IDs: `0x50434452` ("PCDR") in the high word, a range no streamed
/// data uses.
pub(crate) const DROPPED_LOCATOR_BASE: u64 = 0x5043_4452_0000_0000;

/// Whether `locator_id` is one of the PC Object Dropper's.
pub(crate) fn is_dropped_locator(locator_id: u64) -> bool {
    locator_id >> 32 == DROPPED_LOCATOR_BASE >> 32
}

impl LocatorKdTree {
    pub(crate) fn is_empty(&self) -> bool {
        self.root.is_none()
    }
    fn try_new() -> Option<Self> {
        Self::try_with_blocks(KD_NODE_BLOCKS)
    }

    /// Constructor plus `MemStore::Init(0x50, blocks)`.
    pub(crate) fn try_with_blocks(blocks: usize) -> Option<Self> {
        let mut nodes = Vec::new();
        nodes.try_reserve_exact(blocks).ok()?;
        nodes.resize(blocks, None);
        let mut free = Vec::new();
        free.try_reserve_exact(blocks).ok()?;
        free.extend(
            (0..blocks)
                .rev()
                .map(|index| u16::try_from(index).expect("KD blocks index within u16")),
        );
        Some(Self {
            nodes,
            free,
            root: None,
        })
    }

    /// `ClearSpatialLookups` 0x82d36770: `MemStore::Reset` then a null root.
    pub(crate) fn reset(&mut self) {
        let blocks = self.nodes.len();
        self.nodes.iter_mut().for_each(|node| *node = None);
        self.free.clear();
        self.free.extend(
            (0..blocks)
                .rev()
                .map(|index| u16::try_from(index).expect("KD blocks index within u16")),
        );
        self.root = None;
    }

    /// RAIN's KDTree<uint>::FindNearestNeighbour82710ed0 and node82704580
    /// use this same node layout/insertion graph. Equal distances keep the
    /// first visited node; left boundary is inclusive, right is strict.
    pub(crate) fn nearest<N: crate::reckoning_numeric::ReckoningNumeric>(
        &self,
        point: [f32; 2],
        numeric: &mut N,
    ) -> Result<Option<(u64, f32)>, N::Error> {
        let Some(root) = self.root else {
            return Ok(None);
        };
        let mut current = Some(root);
        let mut previous = root;
        let mut back = false;
        let mut dimension = 0;
        let mut best: Option<u64> = None;
        let mut distance = -1.0;
        while let Some(index) = current {
            let node = *self.node(index);
            if !back {
                let dx = node.coord[0] - point[0];
                let dy = node.coord[1] - point[1];
                let squared = dx * dx + dy * dy;
                let candidate = length_from_squared_and_estimate(
                    squared,
                    numeric.reciprocal_sqrt_estimate(squared)?,
                );
                if candidate < distance || best.is_none() {
                    best = Some(node.id);
                    distance = candidate;
                }
            }
            let next = if node.left.is_some()
                && !back
                && !(distance < 0.0)
                && node.coord[dimension] >= point[dimension] - distance
            {
                back = false;
                node.left
            } else if node.right.is_some()
                && (!back || node.left == Some(previous))
                && !(distance < 0.0)
                && point[dimension] + distance > node.coord[dimension]
            {
                back = false;
                node.right
            } else {
                back = true;
                node.parent
            };
            previous = index;
            current = next;
            dimension = (dimension + 1) % 2;
        }
        Ok(best.map(|id| (id, distance)))
    }

    fn node(&self, index: u16) -> &KdNode {
        self.nodes[usize::from(index)]
            .as_ref()
            .expect("linked KD node is live")
    }

    fn node_mut(&mut self, index: u16) -> &mut KdNode {
        self.nodes[usize::from(index)]
            .as_mut()
            .expect("linked KD node is live")
    }

    /// `KDTree::Insert` 0x82d2f000. Its fresh node has null links, so the
    /// post-order subtree walk of `KDNode::Insert` 0x82d2d3f0 visits only it:
    /// ties and NaN descend left, dimensions alternate from 0 at the root.
    pub(crate) fn insert(&mut self, id: u64, coord: [f32; 2]) -> bool {
        let Some(index) = self.free.pop() else {
            return false;
        };
        self.nodes[usize::from(index)] = Some(KdNode {
            left: None,
            right: None,
            parent: None,
            id,
            coord,
        });
        let Some(root) = self.root else {
            self.root = Some(index);
            return true;
        };
        let mut dim = 0;
        let mut parent = root;
        let mut right = false;
        let mut walker = Some(root);
        while let Some(current) = walker {
            parent = current;
            let node = self.node(current);
            right = coord[dim] > node.coord[dim];
            walker = if right { node.right } else { node.left };
            dim = (dim + 1) % 2;
        }
        if right {
            self.node_mut(parent).right = Some(index);
        } else {
            self.node_mut(parent).left = Some(index);
        }
        self.node_mut(index).parent = Some(parent);
        true
    }

    /// `KDNode::FindNear` 0x82d39670: iterative walk from the root, stopping
    /// once `min(neighbours, 256)` nodes are held; no sorting.
    fn find_near_nodes<S: LocatorServices>(
        &self,
        point: [f32; 2],
        radius: f32,
        neighbours: usize,
        services: &mut S,
    ) -> Result<Vec<(u16, f32)>, S::Error> {
        self.find_near_nodes_with(point, radius, neighbours, |squared| {
            services.reciprocal_sqrt_estimate(squared)
        })
    }

    fn find_near_nodes_with<E>(
        &self,
        point: [f32; 2],
        radius: f32,
        neighbours: usize,
        mut estimate: impl FnMut(f32) -> Result<f32, E>,
    ) -> Result<Vec<(u16, f32)>, E> {
        let mut found = Vec::new();
        let Some(root) = self.root else {
            return Ok(found);
        };
        let limit = neighbours.min(KD_SEARCH_CAPACITY);
        let mut node = Some(root);
        let mut previous = root;
        let mut dim = 0;
        let mut back = false;
        while let Some(current) = node {
            if found.len() >= limit {
                break;
            }
            let entry = *self.node(current);
            if !back {
                let dx = entry.coord[0] - point[0];
                let dy = entry.coord[1] - point[1];
                let squared = dx * dx + dy * dy;
                let seed = estimate(squared)?;
                let distance = length_from_squared_and_estimate(squared, seed);
                if !(distance > radius) {
                    found.push((current, distance));
                }
            }
            let next = if entry.left.is_some() && !back && entry.coord[dim] >= point[dim] - radius {
                back = false;
                entry.left
            } else if entry.right.is_some()
                && (!back || entry.left == Some(previous))
                && point[dim] + radius > entry.coord[dim]
            {
                back = false;
                entry.right
            } else {
                back = true;
                entry.parent
            };
            previous = current;
            node = next;
            dim = (dim + 1) % 2;
        }
        Ok(found)
    }

    /// `KDTree::FindNear` 0x82d3a3b8: the node walk, then a distance multiset
    /// (equal distances keep walk order) cut to `max_items`.
    pub(crate) fn find_near<S: LocatorServices>(
        &self,
        point: [f32; 2],
        radius: f32,
        max_items: u32,
        services: &mut S,
    ) -> Result<Vec<(u64, f32)>, LocatorError<S::Error>> {
        self.find_near_with(point, radius, max_items, |squared| {
            services.reciprocal_sqrt_estimate(squared)
        })
    }

    pub(crate) fn find_near_numeric<N: crate::reckoning_numeric::ReckoningNumeric>(
        &self,
        point: [f32; 2],
        radius: f32,
        max_items: u32,
        numeric: &mut N,
    ) -> Result<Vec<(u64, f32)>, LocatorError<N::Error>> {
        self.find_near_with(point, radius, max_items, |squared| {
            numeric.reciprocal_sqrt_estimate(squared)
        })
    }

    fn find_near_with<E>(
        &self,
        point: [f32; 2],
        radius: f32,
        max_items: u32,
        estimate: impl FnMut(f32) -> Result<f32, E>,
    ) -> Result<Vec<(u64, f32)>, LocatorError<E>> {
        if max_items > 256 {
            return Err(LocatorError::FindNearLimit {
                requested: max_items,
            });
        }
        if self.root.is_none() {
            return Ok(Vec::new());
        }
        let limit = usize::try_from(max_items).expect("256 fits usize");
        let found = self
            .find_near_nodes_with(point, radius, limit, estimate)
            .map_err(LocatorError::Services)?;
        if found.iter().any(|(_, distance)| distance.is_nan()) {
            return Err(LocatorError::UnorderedDistance);
        }
        let mut sorted: Vec<(u64, f32)> = found
            .iter()
            .map(|&(index, distance)| (self.node(index).id, distance))
            .collect();
        sorted.sort_by(|a, b| a.1.partial_cmp(&b.1).expect("ordered distances"));
        sorted.truncate(limit);
        Ok(sorted)
    }

    /// `KDTree::Remove` 0x82d3a180: the first four nodes within 0.1 whose
    /// coordinates match within `EPSILON` and whose ID matches.
    fn remove<S: LocatorServices>(
        &mut self,
        id: u64,
        coord: [f32; 2],
        services: &mut S,
    ) -> Result<bool, S::Error> {
        self.remove_matching(Some(id), coord, |squared| {
            services.reciprocal_sqrt_estimate(squared)
        })
        .map(|removed| removed.is_some())
    }

    /// RAIN's coordinate-only overload82725b68 shares the same four-neighbour
    /// search, EPSILON_23 (34000000), and node removal as the locator overload.
    pub(crate) fn remove_coordinate<N: crate::reckoning_numeric::ReckoningNumeric>(
        &mut self,
        coord: [f32; 2],
        numeric: &mut N,
    ) -> Result<Option<u64>, N::Error> {
        self.remove_matching(None, coord, |squared| {
            numeric.reciprocal_sqrt_estimate(squared)
        })
    }

    fn remove_matching<E>(
        &mut self,
        id: Option<u64>,
        coord: [f32; 2],
        estimate: impl FnMut(f32) -> Result<f32, E>,
    ) -> Result<Option<u64>, E> {
        if self.root.is_none() {
            return Ok(None);
        }
        let found = self.find_near_nodes_with(coord, REMOVE_RADIUS, REMOVE_NEIGHBOURS, estimate)?;
        for &(index, _) in found.iter().take(REMOVE_NEIGHBOURS) {
            let node = *self.node(index);
            if abs_bits(coord[0] - node.coord[0]) > REMOVE_EPSILON
                || abs_bits(coord[1] - node.coord[1]) > REMOVE_EPSILON
                || id.is_some_and(|id| node.id != id)
            {
                continue;
            }
            let mut dim = 0;
            let mut parent = node.parent;
            while let Some(ancestor) = parent {
                dim = (dim + 1) % 2;
                parent = self.node(ancestor).parent;
            }
            let removed = self.remove_node(index, dim);
            if self.root == Some(removed) {
                self.root = None;
            }
            self.nodes[usize::from(removed)] = None;
            self.free.push(removed);
            return Ok(Some(node.id));
        }
        Ok(None)
    }

    /// `KDNode<cAssetID>::Remove` 0x82d2d550: a lone left subtree moves right,
    /// the right subtree's minimum in this node's dimension replaces this
    /// object, and that minimum is removed in turn; leaves unlink.
    fn remove_node(&mut self, index: u16, dim: usize) -> u16 {
        let node = self.node_mut(index);
        if node.right.is_none() && node.left.is_some() {
            node.right = node.left;
            node.left = None;
        }
        if let Some(right) = self.node(index).right {
            let mut found_dim = dim;
            let minimum = self.find_min(right, (dim + 1) % 2, dim, &mut found_dim);
            let (id, coord) = {
                let source = self.node(minimum);
                (source.id, source.coord)
            };
            let target = self.node_mut(index);
            target.id = id;
            target.coord = coord;
            return self.remove_node(minimum, found_dim);
        }
        if let Some(parent) = self.node(index).parent {
            let owner = self.node_mut(parent);
            if owner.left == Some(index) {
                owner.left = None;
            } else if owner.right == Some(index) {
                owner.right = None;
            }
        }
        let node = self.node_mut(index);
        node.left = None;
        node.right = None;
        node.parent = None;
        index
    }

    /// `KDNode::FindMin` 0x827eab38: only the left subtree in the minimized
    /// dimension; elsewhere both subtrees, later equal minima winning.
    fn find_min(&self, index: u16, dimstart: usize, mindim: usize, found_dim: &mut usize) -> u16 {
        *found_dim = dimstart;
        let next = (dimstart + 1) % 2;
        let node = *self.node(index);
        if mindim == dimstart {
            return match node.left {
                Some(left) => self.find_min(left, next, mindim, found_dim),
                None => index,
            };
        }
        let mut best = index;
        let mut best_value = node.coord[mindim];
        if let Some(left) = node.left {
            let mut left_dim = dimstart;
            let minimum = self.find_min(left, next, mindim, &mut left_dim);
            let value = self.node(minimum).coord[mindim];
            if !(value > best_value) {
                best_value = value;
                best = minimum;
                *found_dim = left_dim;
            }
        }
        let Some(right) = node.right else {
            return best;
        };
        let mut right_dim = *found_dim;
        let minimum = self.find_min(right, next, mindim, &mut right_dim);
        if self.node(minimum).coord[mindim] > best_value {
            return best;
        }
        *found_dim = right_dim;
        minimum
    }
}

/// `LWDynamicObjectLocatorMan`: retains the native locator map order for
/// ResetAllDynamicObjects. Persistence lookups use the keyed owner.
pub(crate) struct DynamicObjectLocatorMan {
    tree: LocatorKdTree,
    blocks: Vec<Option<DynamicObjectLocator>>,
    free_blocks: Vec<u16>,
    locators: HashMap<u64, u16>,
    /// Constructor82d35110: GetPrevBucketCountOnly(1025) = 953.
    locator_order: Vec<Vec<u64>>,
    persistent: HashMap<u64, PersistentLocatorData>,
    /// `mObjectPersistenceIndexQueue` (`fixed_vector<tObjectPersistenceIndex,
    /// 512>` heap): locator IDs keyed through `GetPersistenceIndex` 0x82d33ce0.
    persistence_queue: Vec<u64>,
    chunk_ids: Vec<u64>,
    /// Locator IDs of every record in the chunks of `mLocatorChunkIDVector`,
    /// the set `GetStreamedLocator` 0x82d2df10 searches. Chunks are never
    /// unregistered by the selected session.
    streamed_locators: HashSet<u64>,
    tick: u32,
}

/// Object state `UpdateObjectPersistentData` 0x82d3c4b0 reads.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ObjectPersistenceInput {
    pub(crate) locator_id: u64,
    /// `GetTransform` (Entity vtable +0x14).
    pub(crate) transform: [f32; 16],
    /// `GetSafetyIndex` (Entity vtable +0x54), read afresh at each use.
    pub(crate) safety: i32,
    /// Characteristics layout +0x64 persistence priority.
    pub(crate) priority: i32,
}

impl DynamicObjectLocatorMan {
    /// Constructor 0x82d3ac40 plus the KD `MemStore::Init(0x50, 0x400)` at
    /// 0x82d3e124 and the locator store 3 it is handed.
    pub(crate) fn try_construct() -> Option<Box<Self>> {
        let tree = LocatorKdTree::try_new()?;
        let mut blocks = Vec::new();
        blocks.try_reserve_exact(LOCATOR_BLOCKS).ok()?;
        blocks.resize(LOCATOR_BLOCKS, None);
        let mut free_blocks = Vec::new();
        free_blocks.try_reserve_exact(LOCATOR_BLOCKS).ok()?;
        free_blocks.extend(
            (0..LOCATOR_BLOCKS)
                .rev()
                .map(|index| u16::try_from(index).expect("1024 locator blocks index within u16")),
        );
        let mut locators = HashMap::new();
        locators.try_reserve(LOCATOR_BLOCKS).ok()?;
        let mut persistent = HashMap::new();
        persistent.try_reserve(PERSISTENT_LOCATORS).ok()?;
        let mut persistence_queue = Vec::new();
        persistence_queue
            .try_reserve_exact(PERSISTENT_LOCATORS)
            .ok()?;
        Some(Box::new(Self {
            tree,
            blocks,
            free_blocks,
            locators,
            locator_order: vec![Vec::new(); 953],
            persistent,
            persistence_queue,
            chunk_ids: Vec::new(),
            streamed_locators: HashSet::new(),
            tick: 0,
        }))
    }

    /// EASTL hash<cAssetID> / DoInsertValue824da510: wrapping high + low.
    fn locator_bucket(id: u64) -> usize {
        ((id as u32).wrapping_add((id >> 32) as u32) % 953) as usize
    }

    /// ResetAllDynamicObjects825697e0 after RequestAllCull(false):
    /// ClearPersistence82d32268, RemoveAllLocators82d3c250 and
    /// RecreateStreamedLocators82d3c2f0. The persistence clock is retained.
    pub(crate) fn reset_all<S: LocatorServices>(&mut self, services: &mut S)
        -> Result<(), LocatorError<S::Error>> {
        self.persistence_queue.clear();
        self.persistent.clear();
        let ids: Vec<_> = self.locator_order.iter().flatten().copied().collect();
        for id in ids { self.destroy_locator(id, services)?; }
        for chunk in self.chunk_ids.clone() {
            self.add_locators_for_streamed_locator_chunk(chunk, 0, true, true, false, services)?;
        }
        Ok(())
    }

    /// `GetLocator` 0x82d33d78 (vtable +0x8 and +0xc).
    pub(crate) fn locator(&self, locator_id: u64) -> Option<&DynamicObjectLocator> {
        let block = *self.locators.get(&locator_id)?;
        self.blocks[usize::from(block)].as_ref()
    }

    pub(crate) fn set_in_session_marker(&mut self,id:u64,in_marker:bool) {
        if let Some(&block)=self.locators.get(&id) {
            if let Some(locator)=self.blocks[usize::from(block)].as_mut() {
                locator.flags=(locator.flags & !LOCATOR_IN_SESSION_MARKER) | if in_marker {LOCATOR_IN_SESSION_MARKER} else {0};
            }
        }
    }

    pub(crate) fn locator_count(&self) -> usize {
        self.locators.len()
    }

    pub(crate) fn chunk_ids(&self) -> &[u64] {
        &self.chunk_ids
    }

    pub(crate) const fn tick(&self) -> u32 {
        self.tick
    }

    /// `GetLocatorPersistenceData` 0x82d33de8.
    pub(crate) fn persistence_data(&self, locator_id: u64) -> Option<PersistentLocatorData> {
        self.persistent.get(&locator_id).copied()
    }

    /// `++mpLocatorMan->mTick` at the head of `UpdatePersistence` 0x82d3d494.
    pub(crate) fn advance_tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    /// `GetStreamedLocator` 0x82d2df10 found a record.
    pub(crate) fn is_streamed_locator(&self, locator_id: u64) -> bool {
        self.streamed_locators.contains(&locator_id)
    }

    /// `AttachObjectToLocator` 0x82d3b7a8 after the object's `SetLocatorID`:
    /// the user ID, removal from the passive tree at the locator's XZ, then
    /// the first attachment tick.
    pub(crate) fn attach_object<S: LocatorServices>(
        &mut self,
        locator_id: u64,
        entity_id: u32,
        services: &mut S,
    ) -> Result<(), LocatorError<S::Error>> {
        let failed = LocatorError::Assert {
            locator_id,
            site: LocatorAssert::AttachUser,
        };
        let Some(&block) = self.locators.get(&locator_id) else {
            return Err(failed);
        };
        let locator = self.blocks[usize::from(block)]
            .as_mut()
            .expect("mapped locator block is live");
        if locator.user_object_id != 0 {
            return Err(failed);
        }
        locator.user_object_id = entity_id;
        let coord = xz(&locator.transform);
        self.tree
            .remove(locator_id, coord, services)
            .map_err(LocatorError::Services)?;
        let tick = self.tick;
        let locator = self.blocks[usize::from(block)]
            .as_mut()
            .expect("mapped locator block is live");
        if locator.attachment_time == 0 {
            locator.attachment_time = tick;
        }
        Ok(())
    }

    /// `DetachObjectFromLocator` 0x82d3b8f0 after the object's
    /// `SetLocatorID(kNullAssetID)`. A persistence locator with neither saved
    /// data nor a streamed record is destroyed; any other returns to the
    /// passive tree at its current XZ. False for a missing or free locator.
    pub(crate) fn detach_object<S: LocatorServices>(
        &mut self,
        locator_id: u64,
        services: &mut S,
    ) -> Result<bool, LocatorError<S::Error>> {
        let Some(&block) = self.locators.get(&locator_id) else {
            return Ok(false);
        };
        let locator = self.blocks[usize::from(block)]
            .as_mut()
            .expect("mapped locator block is live");
        if locator.user_object_id == 0 {
            return Ok(false);
        }
        let flags = locator.flags;
        locator.user_object_id = 0;
        let coord = xz(&locator.transform);
        if flags & LOCATOR_PERSISTENCE != 0
            && !self.persistent.contains_key(&locator_id)
            && !self.streamed_locators.contains(&locator_id)
        {
            self.destroy_locator(locator_id, services)?;
            return Ok(true);
        }
        // `KDTree::Insert`'s full-store false is not read.
        self.tree.insert(locator_id, coord);
        Ok(true)
    }

    /// `DynamicObjectLocator::SetTransform` 0x82d29520: all four rows are
    /// NaN-asserted (line 21), then copied. Attached locators are not in the
    /// passive tree, so no KD entry moves. False for a missing locator.
    pub(crate) fn set_locator_transform<E>(
        &mut self,
        locator_id: u64,
        transform: &[f32; 16],
    ) -> Result<bool, LocatorError<E>> {
        let Some(&block) = self.locators.get(&locator_id) else {
            return Ok(false);
        };
        if !matrix_valid(transform) {
            return Err(LocatorError::Assert {
                locator_id,
                site: LocatorAssert::LocatorTransform,
            });
        }
        self.blocks[usize::from(block)]
            .as_mut()
            .expect("mapped locator block is live")
            .transform = *transform;
        Ok(true)
    }

    /// ResetLocatorTransform82d3c740, after its caller has culled the user.
    pub(crate) fn reset_locator_transform<S: LocatorServices>(&mut self,
        locator_id:u64, transform:&[f32;16], services:&mut S,
    )->Result<(),LocatorError<S::Error>> {
        self.reset_locator_transform_spatial(locator_id, transform, true, services)
    }

    /// `ResetLocatorTransform` 0x82d3c740 after its caller culled the user object (line 1018
    /// asserts none is attached): the transform and the persistence data's transform; the
    /// passive tree entry moves from the old XZ only with `update_spatial_map`.
    pub(crate) fn reset_locator_transform_spatial<S: LocatorServices>(
        &mut self,
        locator_id: u64,
        transform: &[f32; 16],
        update_spatial_map: bool,
        services: &mut S,
    ) -> Result<(), LocatorError<S::Error>> {
        let Some(locator) = self.locator(locator_id) else {
            return Ok(());
        };
        if locator.user_object_id != 0 {
            return Err(LocatorError::Assert {
                locator_id,
                site: LocatorAssert::DestroyUser,
            });
        }
        let old_coord = xz(&locator.transform);
        self.set_locator_transform::<S::Error>(locator_id, transform)?;
        if let Some(mut data) = self.persistent.get(&locator_id).copied() {
            data.basic.transform = data_transform(transform, locator_id, services)?;
            self.set_locator_persistence_data::<S::Error>(locator_id, &data)?;
        }
        if update_spatial_map {
            self.tree
                .remove(locator_id, old_coord, services)
                .map_err(LocatorError::Services)?;
            self.tree.insert(locator_id, xz(transform));
        }
        Ok(())
    }

    /// The locators whose transform puts a 0.1 m sphere (`CreateCollisionSphere`) in a trigger
    /// volume, as `in_volume` answers `TriggerVolumeUtils::IsColldingWithTriggerVolume`, then the
    /// locators of the streamed records whose transform does: the two walks of
    /// `SetLocatorsInTriggerVolumeEnabled` 0x82d32380 and `ResetLocatorsInTriggerVolume`
    /// 0x82d3c928. A first-walk entry carries the locator's initial stream transform, a second-walk
    /// one the record's. The native first walk visits `mLocatorMap` in bucket order; this one
    /// visits the locator blocks.
    fn locators_in_trigger_volume<S: LocatorServices>(
        &self,
        mut in_volume: impl FnMut(&[f32; 16]) -> bool,
        services: &mut S,
    ) -> Result<(Vec<(u64, [f32; 16])>, Vec<(u64, [f32; 16])>), LocatorError<S::Error>> {
        let mut locators = Vec::new();
        for locator in self.blocks.iter().flatten() {
            if !is_dropped_locator(locator.locator_id) && in_volume(&locator.transform) {
                locators.push((locator.locator_id, locator.initial_stream_transform));
            }
        }
        let mut records = Vec::new();
        for &chunk in &self.chunk_ids {
            let chunk_records = services
                .locator_chunk(chunk)
                .map_err(LocatorError::Services)?
                .ok_or(LocatorError::MissingChunk { asset_id: chunk })?;
            for record in &chunk_records {
                if !is_dropped_locator(record.guid) && self.locators.contains_key(&record.guid) && in_volume(&record.transform) {
                    records.push((record.guid, record.transform));
                }
            }
        }
        Ok((locators, records))
    }

    /// `SetLocatorsInTriggerVolumeEnabled` 0x82d32380: the enabled bit of every locator in the
    /// trigger volume (either walk of [`Self::locators_in_trigger_volume`]).
    pub(crate) fn set_locators_in_trigger_volume_enabled<S: LocatorServices>(
        &mut self,
        enabled: bool,
        in_volume: impl FnMut(&[f32; 16]) -> bool,
        services: &mut S,
    ) -> Result<(), LocatorError<S::Error>> {
        let (locators, records) = self.locators_in_trigger_volume(in_volume, services)?;
        for (locator_id, _) in locators.into_iter().chain(records) {
            let Some(&block) = self.locators.get(&locator_id) else {
                continue;
            };
            let flags = &mut self.blocks[usize::from(block)]
                .as_mut()
                .expect("mapped locator block is live")
                .flags;
            *flags = (*flags & !LOCATOR_ENABLED) | if enabled { LOCATOR_ENABLED } else { 0 };
        }
        Ok(())
    }

    /// `ResetLocatorsInTriggerVolume` 0x82d3c928: every locator in the trigger volume has its user
    /// object culled (`cull_user`, `RequestExplicitCull`) and its transform reset without the
    /// passive tree, to its initial stream transform (`ResetLocator(id, false)`) or, from the
    /// streamed records, to the record's; after any reset the passive tree is rebuilt from the
    /// free locators (`MemStore::Reset`, `UpdateSpatialLookups` 0x82d32188, here in block
    /// order).
    pub(crate) fn reset_locators_in_trigger_volume<S: LocatorServices>(
        &mut self,
        in_volume: impl FnMut(&[f32; 16]) -> bool,
        mut cull_user: impl FnMut(&mut Self, u32) -> Result<(), String>,
        services: &mut S,
    ) -> Result<(), LocatorError<S::Error>> {
        let (locators, records) = self.locators_in_trigger_volume(in_volume, services)?;
        let reset = !locators.is_empty() || !records.is_empty();
        for (locator_id, transform) in locators.into_iter().chain(records) {
            let Some(locator) = self.locator(locator_id) else {
                continue;
            };
            let user = locator.user_object_id;
            if user != 0 {
                cull_user(self, user).map_err(LocatorError::CullCallback)?;
            }
            self.reset_locator_transform_spatial(locator_id, &transform, false, services)?;
        }
        if reset {
            self.tree.reset();
            for locator in self.blocks.iter().flatten() {
                if locator.user_object_id == 0 {
                    self.tree.insert(locator.locator_id, xz(&locator.transform));
                }
            }
        }
        Ok(())
    }

    /// `ResetStreamedLocatorData` 0x82d2de58: `ResetLocator(id, false)` of each record's locator,
    /// its user object culled (`cull_user`, `RequestExplicitCullOfUserObject`) and its transform
    /// back to the initial stream transform, the passive tree untouched.
    pub(crate) fn reset_streamed_locator_data<S: LocatorServices>(
        &mut self,
        asset_id: u64,
        mut cull_user: impl FnMut(&mut Self, u32) -> Result<(), String>,
        services: &mut S,
    ) -> Result<(), LocatorError<S::Error>> {
        let records = services
            .locator_chunk(asset_id)
            .map_err(LocatorError::Services)?
            .ok_or(LocatorError::MissingChunk { asset_id })?;
        for record in &records {
            let Some(locator) = self.locator(record.guid) else {
                continue;
            };
            let (user, transform) = (locator.user_object_id, locator.initial_stream_transform);
            if user != 0 {
                cull_user(self, user).map_err(LocatorError::CullCallback)?;
            }
            self.reset_locator_transform_spatial(record.guid, &transform, false, services)?;
        }
        Ok(())
    }

    /// `SerializeLocatorsInTriggerVolume` 0x82d35ba0: the `tLocatorData` of each locator in the
    /// trigger volume (first walk of [`Self::locators_in_trigger_volume`]): its transform, initial
    /// stream transform, IDs and attachment time.
    pub(crate) fn serialize_locators_in_trigger_volume<S: LocatorServices>(
        &self,
        mut in_volume: impl FnMut(&[f32; 16]) -> bool,
        services: &mut S,
    ) -> Result<Vec<LocatorData>, LocatorError<S::Error>> {
        let mut list = Vec::new();
        for locator in self.blocks.iter().flatten() {
            if is_dropped_locator(locator.locator_id) || !in_volume(&locator.transform) {
                continue;
            }
            let locator_id = locator.locator_id;
            let transform = data_transform(&locator.transform, locator_id, services)?;
            if !matrix_valid(&locator.initial_stream_transform) {
                return Err(LocatorError::Assert {
                    locator_id,
                    site: LocatorAssert::DataInitialStreamTransform,
                });
            }
            let initial_stream_transform =
                quat_pos_from_matrix(&locator.initial_stream_transform, services)
                    .map_err(LocatorError::Services)?;
            list.push(LocatorData {
                transform,
                initial_stream_transform,
                locator_id,
                model_id: locator.model_id,
                attachment_time: locator.attachment_time,
            });
        }
        Ok(list)
    }

    /// `RestoreLocatorsInSerializedList` 0x82d3b650 as `RestoreFromSavedState(-1, buffer)` calls it,
    /// with no serialized list: a saved locator still present is enabled again; a missing one
    /// with persistence data or a streamed record (`GetStreamedLocator`) is created from the saved
    /// data.
    pub(crate) fn restore_saved_locators<S: LocatorServices>(
        &mut self,
        saved: &[LocatorData],
        services: &mut S,
    ) -> Result<(), LocatorError<S::Error>> {
        for data in saved {
            let locator_id = data.locator_id;
            if let Some(&block) = self.locators.get(&locator_id) {
                self.blocks[usize::from(block)]
                    .as_mut()
                    .expect("mapped locator block is live")
                    .flags |= LOCATOR_ENABLED;
            } else if self.persistent.contains_key(&locator_id)
                || self.is_streamed_locator(locator_id)
            {
                self.create_locator(locator_id, data, services)?;
            }
        }
        Ok(())
    }

    /// `DestroyLocator` 0x82d3b0c0.
    fn destroy_locator<S: LocatorServices>(
        &mut self,
        locator_id: u64,
        services: &mut S,
    ) -> Result<(), LocatorError<S::Error>> {
        let Some(&block) = self.locators.get(&locator_id) else {
            return Ok(());
        };
        let locator = self.blocks[usize::from(block)]
            .as_ref()
            .expect("mapped locator block is live");
        if locator.user_object_id != 0 {
            return Err(LocatorError::Assert {
                locator_id,
                site: LocatorAssert::DestroyUser,
            });
        }
        let coord = xz(&locator.transform);
        self.tree
            .remove(locator_id, coord, services)
            .map_err(LocatorError::Services)?;
        self.locators.remove(&locator_id);
        self.locator_order[Self::locator_bucket(locator_id)].retain(|id| *id != locator_id);
        self.blocks[usize::from(block)] = None;
        self.free_blocks.push(block);
        Ok(())
    }

    /// `tObjectPersistenceIndex::GetPersistenceIndex` 0x82d33ce0:
    /// `mullw(mullw(safety, priority), attachTime)`, or -1 without data.
    fn persistence_index(&self, locator_id: u64) -> i32 {
        self.persistent.get(&locator_id).map_or(-1, |data| {
            data.safety
                .wrapping_mul(data.priority)
                .wrapping_mul(i32::from_ne_bytes(data.basic.attachment_time.to_ne_bytes()))
        })
    }

    /// `promote_heap` 0x82d374b0: parents with a greater (signed) index move
    /// down, so the queue top holds the smallest index.
    fn persistence_promote(&mut self, top: usize, mut position: usize, value: u64) {
        let value_index = self.persistence_index(value);
        while position > top {
            let parent = (position - 1) >> 1;
            if self.persistence_index(self.persistence_queue[parent]) <= value_index {
                break;
            }
            self.persistence_queue[position] = self.persistence_queue[parent];
            position = parent;
        }
        self.persistence_queue[position] = value;
    }

    /// `adjust_heap` 0x82d38668.
    fn persistence_adjust(&mut self, top: usize, size: usize, mut position: usize, value: u64) {
        let mut child = 2 * (position + 1);
        while child < size {
            if self.persistence_index(self.persistence_queue[child])
                > self.persistence_index(self.persistence_queue[child - 1])
            {
                child -= 1;
            }
            self.persistence_queue[position] = self.persistence_queue[child];
            position = child;
            child = 2 * (child + 1);
        }
        if child == size {
            self.persistence_queue[position] = self.persistence_queue[child - 1];
            position = child - 1;
        }
        self.persistence_promote(top, position, value);
    }

    /// `priority_queue::remove` 0x82d3a578.
    fn persistence_queue_remove(&mut self, position: usize) {
        let size = self.persistence_queue.len();
        let removed = self.persistence_queue[position];
        let last = self.persistence_queue[size - 1];
        self.persistence_queue[size - 1] = removed;
        self.persistence_adjust(0, size - 1, position, last);
        self.persistence_queue.pop();
    }

    /// `change_heap` 0x82d3a6f0.
    fn persistence_queue_change(&mut self, position: usize) {
        let size = self.persistence_queue.len();
        let last = self.persistence_queue[size - 1];
        self.persistence_queue[size - 1] = self.persistence_queue[position];
        self.persistence_adjust(0, size - 1, position, last);
        let moved = self.persistence_queue[size - 1];
        self.persistence_promote(0, size - 1, moved);
    }

    /// `AddLocatorToPersistence` 0x82d3a098: map insert, then queue push.
    fn add_locator_to_persistence<E>(
        &mut self,
        locator_id: u64,
        data: PersistentLocatorData,
    ) -> Result<bool, LocatorError<E>> {
        if self.persistent.contains_key(&locator_id) {
            return Ok(false);
        }
        if self.persistent.len() >= PERSISTENT_LOCATORS
            || self.persistence_queue.len() >= PERSISTENT_LOCATORS
        {
            // fixed_hash_map/fixed_vector<512> without overflow.
            return Err(LocatorError::Allocation);
        }
        self.persistent.insert(locator_id, data);
        self.persistence_queue.push(locator_id);
        let last = self.persistence_queue.len() - 1;
        self.persistence_promote(0, last, locator_id);
        Ok(true)
    }

    /// `RemoveLocatorFromPersistence` 0x82d3ba48: queue removal while the
    /// map entry still feeds the index, map erase, then an unattached,
    /// unstreamed locator is destroyed.
    fn remove_locator_from_persistence<S: LocatorServices>(
        &mut self,
        locator_id: u64,
        services: &mut S,
    ) -> Result<bool, LocatorError<S::Error>> {
        if !self.persistent.contains_key(&locator_id) {
            return Ok(false);
        }
        let failed = LocatorError::Assert {
            locator_id,
            site: LocatorAssert::PersistenceQueue,
        };
        let Some(position) = self
            .persistence_queue
            .iter()
            .position(|&queued| queued == locator_id)
        else {
            return Err(failed);
        };
        self.persistence_queue_remove(position);
        self.persistent.remove(&locator_id);
        let Some(&block) = self.locators.get(&locator_id) else {
            return Err(failed);
        };
        let user = self.blocks[usize::from(block)]
            .as_ref()
            .expect("mapped locator block is live")
            .user_object_id;
        if user == 0 && !self.streamed_locators.contains(&locator_id) {
            self.destroy_locator(locator_id, services)?;
        }
        Ok(true)
    }

    /// `SetLocatorPersistenceData` 0x82d3c350: every field is stored; a
    /// changed safety re-heaps the entry (`change_heap`).
    fn set_locator_persistence_data<E>(
        &mut self,
        locator_id: u64,
        data: &PersistentLocatorData,
    ) -> Result<bool, LocatorError<E>> {
        let Some(stored) = self.persistent.get_mut(&locator_id) else {
            return Ok(false);
        };
        let old_safety = stored.safety;
        *stored = *data;
        if old_safety != data.safety {
            let Some(position) = self
                .persistence_queue
                .iter()
                .position(|&queued| queued == locator_id)
            else {
                return Err(LocatorError::Assert {
                    locator_id,
                    site: LocatorAssert::PersistenceQueue,
                });
            };
            self.persistence_queue_change(position);
        }
        Ok(true)
    }

    /// `UpdateObjectPersistentData` 0x82d3c4b0 for one object whose locator ID
    /// is not `kNullAssetID`.
    pub(crate) fn update_object_persistent_data<S: LocatorServices>(
        &mut self,
        input: ObjectPersistenceInput,
        services: &mut S,
    ) -> Result<(), LocatorError<S::Error>> {
        let locator_id = input.locator_id;
        let Some(&block) = self.locators.get(&locator_id) else {
            return Err(LocatorError::Assert {
                locator_id,
                site: LocatorAssert::PersistenceLocator,
            });
        };
        let locator = self.blocks[usize::from(block)]
            .as_ref()
            .expect("mapped locator block is live");
        if locator.flags & LOCATOR_PERSISTENCE == 0 {
            return Ok(());
        }
        let initial_stream_transform = locator.initial_stream_transform;
        let model_id = locator.model_id;
        let attachment_time = locator.attachment_time;
        let existing = self.persistence_data(locator_id);
        let found = existing.is_some();
        let mut data = if let Some(data) = existing {
            data
        } else {
            let transform = data_transform(&input.transform, locator_id, services)?;
            if !matrix_valid(&initial_stream_transform) {
                return Err(LocatorError::Assert {
                    locator_id,
                    site: LocatorAssert::DataInitialStreamTransform,
                });
            }
            let initial_stream_transform =
                quat_pos_from_matrix(&initial_stream_transform, services)
                    .map_err(LocatorError::Services)?;
            PersistentLocatorData {
                basic: LocatorData {
                    transform,
                    initial_stream_transform,
                    locator_id,
                    model_id,
                    attachment_time,
                },
                priority: input.priority,
                safety: input.safety,
            }
        };
        let remove = input.priority == 0 || input.safety == 0 || input.safety == -1;
        if found {
            if remove {
                self.remove_locator_from_persistence(locator_id, services)?;
                return Ok(());
            }
            data.basic.transform = data_transform(&input.transform, locator_id, services)?;
            data.safety = input.safety;
            self.set_locator_persistence_data::<S::Error>(locator_id, &data)?;
            return Ok(());
        }
        if remove {
            return Ok(());
        }
        let mut add = true;
        if self.persistent.len() >= PERSISTENT_LOCATORS {
            add = false;
            let top = self.persistence_queue[0];
            let index = data
                .priority
                .wrapping_mul(i32::from_ne_bytes(data.basic.attachment_time.to_ne_bytes()))
                .wrapping_mul(data.safety);
            if input.priority == PERSISTENCE_PRIORITY_EVICT || index > self.persistence_index(top) {
                add = self.remove_locator_from_persistence(top, services)?;
            }
        }
        if add {
            self.add_locator_to_persistence::<S::Error>(locator_id, data)?;
        }
        Ok(())
    }

    /// `GetPassiveLocatorsWithinRange` 0x82d3ad20.
    pub(crate) fn passive_locators_within_range<S: LocatorServices>(
        &self,
        point: [f32; 2],
        radius: f32,
        max_entities: u32,
        services: &mut S,
    ) -> Result<Vec<(u64, f32)>, LocatorError<S::Error>> {
        let max_items = if max_entities < PASSIVE_QUERY_CLAMP {
            max_entities
        } else {
            PASSIVE_QUERY_CLAMP
        };
        self.tree.find_near(point, radius, max_items, services)
    }

    /// `AddStreamedLocatorData` 0x82d3d548.
    pub(crate) fn add_streamed_locator_data<S: LocatorServices>(
        &mut self,
        asset_id: u64,
        asset_flags: u32,
        allow_persistence: bool,
        allow_priority_tracking: bool,
        treat_as_static: bool,
        services: &mut S,
    ) -> Result<(), LocatorError<S::Error>> {
        if !services
            .enable_locator_loading()
            .map_err(LocatorError::Services)?
        {
            return Ok(());
        }
        let records = self.add_locators_for_streamed_locator_chunk(
            asset_id,
            asset_flags,
            allow_persistence,
            allow_priority_tracking,
            treat_as_static,
            services,
        )?;
        if !self.chunk_ids.contains(&asset_id) {
            self.chunk_ids
                .try_reserve(1)
                .map_err(|_| LocatorError::Allocation)?;
            self.streamed_locators
                .try_reserve(records.len())
                .map_err(|_| LocatorError::Allocation)?;
            self.chunk_ids.push(asset_id);
            self.streamed_locators.extend(records);
        }
        Ok(())
    }

    /// PC Object Dropper: one enabled locator for a dropped prop, made as a streamed chunk's
    /// record is (`AddLocatorsForStreamedLocatorChunk`) but belonging to no chunk, so streaming
    /// never removes it. The census then spawns, culls and respawns its object like any city
    /// prop, and a moved object saves its transform to it (`UpdatePostTransforms`). False when
    /// the locator store is full.
    pub(crate) fn add_dropped_locator<S: LocatorServices>(
        &mut self,
        locator_id: u64,
        model_id: u64,
        transform: &[f32; 16],
        services: &mut S,
    ) -> Result<bool, LocatorError<S::Error>> {
        if !matrix_valid(transform) {
            return Err(LocatorError::Assert { locator_id, site: LocatorAssert::DataTransform });
        }
        let quat_pos = quat_pos_from_matrix(transform, services).map_err(LocatorError::Services)?;
        let data = LocatorData {
            transform: quat_pos,
            initial_stream_transform: quat_pos,
            locator_id,
            model_id,
            attachment_time: 0,
        };
        let Some(block) = self.create_locator(locator_id, &data, services)? else {
            return Ok(false);
        };
        let flags = &mut self.blocks[usize::from(block)].as_mut().expect("created locator block").flags;
        *flags = (*flags & CHUNK_FLAG_KEEP) | LOCATOR_ENABLED;
        Ok(true)
    }

    /// PC Object Dropper: a dropped prop's locator goes. Its object must be culled first.
    pub(crate) fn remove_dropped_locator<S: LocatorServices>(
        &mut self,
        locator_id: u64,
        services: &mut S,
    ) -> Result<(), LocatorError<S::Error>> {
        self.destroy_locator(locator_id, services)
    }

    /// RemoveStreamedLocatorData82d3d618 / RemoveLocatorsForStreamedLocatorChunk
    /// 82d3c0c8. Active objects survive ordinary streaming; passive locators
    /// survive only the native persistence rules. Chunk membership is erased
    /// AFTER the loop so explicit culls see the same DetachObject state.
    pub(crate) fn remove_streamed_locator_data<S: LocatorServices>(
        &mut self,
        asset_id: u64,
        asset_flags: u32,
        objects: &mut super::dmo_objects::DmoObjectMan,
        services: &mut S,
    ) -> Result<(), LocatorError<S::Error>> {
        self.remove_streamed_locator_data_with_monitors(asset_id,asset_flags,objects,services,|_,monitor,_|
            Err(format!("streaming monitor {monitor:#x} requires physical-world callback")))
    }
    pub(crate) fn remove_streamed_locator_data_with_monitors<S:LocatorServices>(
        &mut self,asset_id:u64,asset_flags:u32,objects:&mut super::dmo_objects::DmoObjectMan,
        services:&mut S,mut notify:impl FnMut(&mut super::dmo_objects::DmoObjectMan,u32,u32)->Result<(),String>,
    )->Result<(),LocatorError<S::Error>>{
        // LWDynamicObjectMan82d3d6e8 supplies force=false, persistence=true.
        self.remove_streamed_locator_data_with_lifecycle(asset_id,asset_flags,false,true,objects,services,|objects,_,event|{
            match event {
                super::dmo_objects::DmoCullEvent::BeforeCull(id)=>{
                    if objects.object(id).is_some_and(|o|!o.contained_props.is_empty()) {
                        Err("streamed DMO child props require synchronous before-cull services".into())
                    } else {Ok(())}
                }
                super::dmo_objects::DmoCullEvent::Monitor{entity,monitor}=>notify(objects,monitor,entity),
            }
        })
    }
    /// The flags' bit 7 forces the user objects' cull as `force_object_cull` does; bit 5 keeps
    /// persistence locators from `allow_persistence`.
    #[expect(clippy::too_many_arguments, reason = "RemoveStreamedLocatorData's own arguments")]
    pub(crate) fn remove_streamed_locator_data_with_lifecycle<S:LocatorServices>(
        &mut self,asset_id:u64,asset_flags:u32,force_object_cull:bool,allow_persistence:bool,
        objects:&mut super::dmo_objects::DmoObjectMan,
        services:&mut S,mut notify:impl FnMut(&mut super::dmo_objects::DmoObjectMan,&mut Self,super::dmo_objects::DmoCullEvent)->Result<(),String>,
    )->Result<(),LocatorError<S::Error>>{
        let records = services
            .locator_chunk(asset_id)
            .map_err(LocatorError::Services)?
            .ok_or(LocatorError::MissingChunk { asset_id })?;
        let force = asset_flags & 0x80 != 0 || force_object_cull;
        let persistence = allow_persistence && asset_flags & 0x20 == 0;
        for record in &records {
            let Some(&block) = self.locators.get(&record.guid) else {
                continue;
            };
            if force {
                let user = self.blocks[usize::from(block)]
                    .as_ref()
                    .expect("live locator")
                    .user_object_id;
                if user != 0 {
                    objects.try_request_explicit_cull_with_lifecycle(user, self, services,&mut notify)?;
                }
            }
            let Some(&block) = self.locators.get(&record.guid) else {
                continue;
            };
            let locator = self.blocks[usize::from(block)]
                .as_ref()
                .expect("live locator");
            if locator.user_object_id == 0
                && (!persistence
                    || (locator.flags & LOCATOR_PERSISTENCE != 0
                        && self.persistence_data(record.guid).is_none()))
            {
                self.destroy_locator(record.guid, services)?;
            }
        }
        if let Some(index) = self.chunk_ids.iter().position(|id| *id == asset_id) {
            self.chunk_ids.remove(index);
            for record in &records {
                self.streamed_locators.remove(&record.guid);
            }
            // This set accelerates native IsLocatorStreamed's chunk scan.
            // A GUID also authored in another resident chunk is still streamed.
            for chunk in &self.chunk_ids {
                let remaining = services
                    .locator_chunk(*chunk)
                    .map_err(LocatorError::Services)?
                    .ok_or(LocatorError::MissingChunk { asset_id: *chunk })?;
                self.streamed_locators
                    .extend(remaining.iter().map(|record| record.guid));
            }
        }
        Ok(())
    }

    /// `AddLocatorsForStreamedLocatorChunk` 0x82d3be90, one 128-byte record at
    /// a time: locator ID +0x60, model ID +0x70, transform +0 for both data
    /// transforms, attachment time 0. Returns every record's locator ID.
    fn add_locators_for_streamed_locator_chunk<S: LocatorServices>(
        &mut self,
        asset_id: u64,
        asset_flags: u32,
        allow_persistence: bool,
        allow_priority_tracking: bool,
        treat_as_static: bool,
        services: &mut S,
    ) -> Result<Vec<u64>, LocatorError<S::Error>> {
        let is_static = (asset_flags >> 4) & 1 != 0 || treat_as_static;
        let persistence = allow_persistence && (asset_flags >> 5) & 1 == 0;
        let priority = allow_priority_tracking && (asset_flags >> 6) & 1 == 0;
        let records = services
            .locator_chunk(asset_id)
            .map_err(LocatorError::Services)?
            .ok_or(LocatorError::MissingChunk { asset_id })?;
        for record in &records {
            let locator_id = record.guid;
            if !matrix_valid(&record.transform) {
                return Err(LocatorError::Assert {
                    locator_id,
                    site: LocatorAssert::DataTransform,
                });
            }
            let transform = quat_pos_from_matrix(&record.transform, services)
                .map_err(LocatorError::Services)?;
            let initial_stream_transform = quat_pos_from_matrix(&record.transform, services)
                .map_err(LocatorError::Services)?;
            let mut data = LocatorData {
                transform,
                initial_stream_transform,
                locator_id,
                model_id: record.instance_guid,
                attachment_time: 0,
            };
            let existing = self.locators.get(&locator_id).copied();
            let mut create = true;
            if persistence {
                if existing.is_some() {
                    create = false;
                } else if let Some(persisted) = self.persistence_data(locator_id) {
                    data = persisted.basic;
                }
            }
            let locator = if create {
                let Some(block) = self.create_locator(locator_id, &data, services)? else {
                    continue;
                };
                let flags = &mut self.blocks[usize::from(block)]
                    .as_mut()
                    .expect("created locator block")
                    .flags;
                *flags = (*flags & CHUNK_FLAG_KEEP)
                    | if is_static { LOCATOR_STATIC } else { 0 }
                    | if priority {
                        LOCATOR_PRIORITY_TRACKING
                    } else {
                        0
                    }
                    | if persistence { LOCATOR_PERSISTENCE } else { 0 };
                block
            } else {
                existing.expect("persistence kept an existing locator")
            };
            self.blocks[usize::from(locator)]
                .as_mut()
                .expect("kept locator block")
                .flags |= LOCATOR_ENABLED;
        }
        let mut ids = Vec::new();
        ids.try_reserve_exact(records.len())
            .map_err(|_| LocatorError::Allocation)?;
        ids.extend(records.iter().map(|record| record.guid));
        Ok(ids)
    }

    /// `CreateLocator` 0x82d3ad38. An attached locator (`mUserObjectID`) is
    /// returned untouched; otherwise transforms, model and attachment time are
    /// rewritten and the KD entry moves from the old to the new XZ.
    fn create_locator<S: LocatorServices>(
        &mut self,
        locator_id: u64,
        data: &LocatorData,
        services: &mut S,
    ) -> Result<Option<u16>, LocatorError<S::Error>> {
        if services
            .remove_bad_locators()
            .map_err(LocatorError::Services)?
            && BAD_LOCATOR_IDS.contains(&locator_id)
        {
            return Ok(None);
        }
        let failed = |site| LocatorError::Assert { locator_id, site };
        if !xyz_valid(data.transform.pos) {
            return Err(failed(LocatorAssert::CreatePosition));
        }
        if data.transform.quat.iter().any(|lane| lane.is_nan()) {
            return Err(failed(LocatorAssert::CreateQuaternion));
        }
        let transform = matrix_from_quat_pos(&data.transform);
        if !matrix_valid(&transform) {
            return Err(failed(LocatorAssert::CreateTransform));
        }
        let (block, created) = if let Some(&block) = self.locators.get(&locator_id) {
            (block, false)
        } else {
            let Some(block) = self.free_blocks.pop() else {
                return Ok(None);
            };
            self.blocks[usize::from(block)] = Some(DynamicObjectLocator {
                transform: [0.0; 16],
                initial_stream_transform: [0.0; 16],
                locator_id,
                model_id: data.model_id,
                attachment_time: 0,
                user_object_id: 0,
                flags: NEW_LOCATOR_FLAGS,
            });
            (block, true)
        };
        if self.blocks[usize::from(block)]
            .as_ref()
            .expect("found locator block is live")
            .user_object_id
            != 0
        {
            return Ok(Some(block));
        }
        let initial_stream_transform = matrix_from_quat_pos(&data.initial_stream_transform);
        if !matrix_valid(&initial_stream_transform) {
            if created {
                self.blocks[usize::from(block)] = None;
                self.free_blocks.push(block);
            }
            return Err(failed(LocatorAssert::LocatorInitialStreamTransform));
        }
        let locator = self.blocks[usize::from(block)]
            .as_mut()
            .expect("found locator block is live");
        let old_coord = xz(&locator.transform);
        locator.transform = transform;
        locator.initial_stream_transform = initial_stream_transform;
        locator.model_id = data.model_id;
        locator.attachment_time = data.attachment_time;
        if created {
            self.locators.insert(locator_id, block);
            self.locator_order[Self::locator_bucket(locator_id)].insert(0, locator_id);
        } else {
            self.tree
                .remove(locator_id, old_coord, services)
                .map_err(LocatorError::Services)?;
        }
        self.tree.insert(locator_id, xz(&transform));
        Ok(Some(block))
    }
}

#[cfg(test)]
#[path = "living_world_dmo_locators_tests.rs"]
mod tests;
