//! Runtime stream-focus membership and collection lifecycle.

use std::{
    fmt,
    hash::{Hash, Hasher},
    sync::Arc,
};

use skate2_formats::stream::FocusCollection;

#[path = "area_loaded.rs"]
pub mod area_loaded;
#[path = "replay_focus.rs"]
pub mod replay_focus;

/// Exact value installed in `assman::kInvalidCollectionID` by the dynamic
/// initializer at `0x82f760c0`.
///
/// The reference derives this as the `RenderWare` hash of `InvalidStringHash`.
pub const INVALID_COLLECTION_ID: u64 = 0xe171_e1ab_fedb_69ed;

/// Stable identity of one stream focus.
///
/// Native requests retain the originating `tStreamFocus*` and
/// `cAssetStreamSystem::UpdateLoadRequest` compares that pointer at
/// `0x824d7368..0x824d737c`. A private [`Arc`] allocation is a high-confidence
/// safe host representation of the same identity relationship; it is not a
/// guest address.
#[derive(Clone)]
pub struct FocusHandle(Arc<()>);

impl FocusHandle {
    /// Creates a distinct live focus identity.
    #[must_use]
    pub fn new() -> Self {
        Self(Arc::new(()))
    }
}

impl Default for FocusHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for FocusHandle {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for FocusHandle {}

impl Hash for FocusHandle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.0).hash(state);
    }
}

impl fmt::Debug for FocusHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("FocusHandle")
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StreamFocusEntry {
    pub id: u64,
    pub position: [f32; 3],
    pub distance_xz: f32,
    pub stream_order: i32,
}

impl From<FocusCollection> for StreamFocusEntry {
    fn from(value: FocusCollection) -> Self {
        Self {
            id: value.id,
            position: value.position,
            distance_xz: value.distance_xz,
            stream_order: value.stream_order,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollectionRequestKind {
    Load,
    Update,
    Unload,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CollectionRequest {
    pub handle: FocusHandle,
    pub kind: CollectionRequestKind,
    pub id: u64,
    pub distance_xz: f32,
    pub stream_order: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StreamFocusTransition {
    pub previous_current_id: Option<u64>,
    pub current_id: Option<u64>,
    pub current_changed: bool,
    /// Requests in the asset-ID merge order used by the reference.
    pub requests: Vec<CollectionRequest>,
}

/// Readable reconstruction of the collection-membership part of
/// `tStreamFocus`.
#[derive(Clone, Debug, PartialEq)]
pub struct StreamFocusState {
    handle: FocusHandle,
    position: [f32; 3],
    locked: bool,
    dirty: bool,
    current_id: Option<u64>,
    /// Kept in ascending asset-ID order, matching `ProcessEntries` at
    /// `0x824deb60`.
    entries: Vec<StreamFocusEntry>,
}

impl StreamFocusState {
    #[must_use]
    pub fn new(position: [f32; 3], locked: bool) -> Self {
        Self {
            handle: FocusHandle::new(),
            position,
            locked,
            dirty: true,
            current_id: None,
            entries: Vec::new(),
        }
    }

    /// Returns the stable identity carried by this focus's collection
    /// requests.
    #[must_use]
    pub const fn handle(&self) -> &FocusHandle {
        &self.handle
    }

    #[must_use]
    pub const fn position(&self) -> [f32; 3] {
        self.position
    }

    #[must_use]
    pub const fn is_locked(&self) -> bool {
        self.locked
    }

    #[must_use]
    pub const fn is_dirty(&self) -> bool {
        self.dirty
    }

    #[must_use]
    pub const fn current_id(&self) -> Option<u64> {
        self.current_id
    }

    #[must_use]
    pub fn entries(&self) -> &[StreamFocusEntry] {
        &self.entries
    }

    /// Reconstructs the position/lock behavior of
    /// `cAssetStreamSystem::MoveStreamFocus` at `0x824d78b8`.
    ///
    /// An unlocked move is accepted. A locked focus moves only when the caller
    /// also requests a lock. As in the reference, the lock is acquired inside
    /// the changed-position branch.
    pub fn move_to(&mut self, position: [f32; 3], lock: bool) -> bool {
        if self.locked && !lock || same_position(self.position, position) {
            return false;
        }
        self.position = position;
        self.dirty = true;
        if lock {
            self.locked = true;
        }
        true
    }

    /// Diffs a distance-ordered focus result against current membership.
    ///
    /// `tStreamFocus::FindCollections` records the nearest entry as current
    /// before `ProcessEntries` sorts the set by 64-bit asset ID. The latter then
    /// performs one merge pass, emitting unloads, loads, and updates. This
    /// method preserves that observable order while leaving asynchronous I/O
    /// scheduling to the native asset service.
    pub fn reconcile(&mut self, selected: &[FocusCollection]) -> StreamFocusTransition {
        let previous_current_id = self.current_id;
        let current_id = selected.first().map(|entry| entry.id);
        let mut next = selected
            .iter()
            .copied()
            .map(StreamFocusEntry::from)
            .collect::<Vec<_>>();
        next.sort_by_key(|entry| entry.id);
        debug_assert!(next.windows(2).all(|pair| pair[0].id != pair[1].id));

        let mut requests = Vec::with_capacity(self.entries.len() + next.len());
        let (mut old_index, mut new_index) = (0, 0);
        while old_index < self.entries.len() || new_index < next.len() {
            match (self.entries.get(old_index), next.get(new_index)) {
                (Some(old), Some(new)) if old.id < new.id => {
                    requests.push(CollectionRequest {
                        handle: self.handle.clone(),
                        kind: CollectionRequestKind::Unload,
                        id: old.id,
                        distance_xz: old.distance_xz,
                        stream_order: old.stream_order,
                    });
                    old_index += 1;
                }
                (Some(old), Some(new)) if new.id < old.id => {
                    requests.push(CollectionRequest {
                        handle: self.handle.clone(),
                        kind: CollectionRequestKind::Load,
                        id: new.id,
                        distance_xz: new.distance_xz,
                        stream_order: new.stream_order,
                    });
                    new_index += 1;
                }
                (Some(_), Some(new)) => {
                    requests.push(CollectionRequest {
                        handle: self.handle.clone(),
                        kind: CollectionRequestKind::Update,
                        id: new.id,
                        distance_xz: new.distance_xz,
                        stream_order: new.stream_order,
                    });
                    old_index += 1;
                    new_index += 1;
                }
                (Some(old), None) => {
                    requests.push(CollectionRequest {
                        handle: self.handle.clone(),
                        kind: CollectionRequestKind::Unload,
                        id: old.id,
                        distance_xz: old.distance_xz,
                        stream_order: old.stream_order,
                    });
                    old_index += 1;
                }
                (None, Some(new)) => {
                    requests.push(CollectionRequest {
                        handle: self.handle.clone(),
                        kind: CollectionRequestKind::Load,
                        id: new.id,
                        distance_xz: new.distance_xz,
                        stream_order: new.stream_order,
                    });
                    new_index += 1;
                }
                (None, None) => break,
            }
        }

        self.current_id = current_id;
        self.entries = next;
        self.dirty = false;
        StreamFocusTransition {
            previous_current_id,
            current_id,
            current_changed: previous_current_id != current_id,
            requests,
        }
    }
}

#[allow(clippy::float_cmp)] // The reference uses exact vector equality here.
fn same_position(left: [f32; 3], right: [f32; 3]) -> bool {
    left[0] == right[0] && left[1] == right[1] && left[2] == right[2]
}

/// Exact `f32` seed loaded by `cAssetStreamSystem::LoadBestCollection`
/// (`0x824dbdac`, data word `0x4e6e6b28`).
const BEST_LOAD_DISTANCE_SENTINEL: f32 = 1_000_000_000.0;

/// Pending operations for one reference stream type.
///
/// This reconstructs `AddLoadRequest`, `AddUnloadRequest`,
/// `UpdateLoadRequest`, `LoadBestCollection`, `LoadNextCollection`, and
/// `UnloadNextCollection`. The platform I/O service owns the active operation;
/// this queue only determines which request is dispatched next.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CollectionRequestQueue {
    loads: Vec<CollectionRequest>,
    unloads: Vec<CollectionRequest>,
}

impl CollectionRequestQueue {
    #[must_use]
    pub fn pending_loads(&self) -> &[CollectionRequest] {
        &self.loads
    }

    #[must_use]
    pub fn pending_unloads(&self) -> &[CollectionRequest] {
        &self.unloads
    }

    pub fn apply_transition(&mut self, transition: &StreamFocusTransition) {
        for request in &transition.requests {
            match request.kind {
                CollectionRequestKind::Load => self.add_load(request.clone()),
                CollectionRequestKind::Update => self.update_load(request),
                CollectionRequestKind::Unload => self.add_unload(request.clone()),
            }
        }
    }

    /// Ignores the exact invalid ID before any mutation, then cancels a pending
    /// opposite request or inserts by ascending stream order. The early guard
    /// is `0x824dda30..0x824dda3c` in the reference.
    pub fn add_load(&mut self, request: CollectionRequest) {
        if request.id == INVALID_COLLECTION_ID {
            return;
        }
        debug_assert_eq!(request.kind, CollectionRequestKind::Load);
        if let Some(index) = self
            .unloads
            .iter()
            .position(|pending| pending.id == request.id)
        {
            self.unloads.remove(index);
            return;
        }
        let index = self
            .loads
            .iter()
            .position(|pending| pending.stream_order > request.stream_order)
            .unwrap_or(self.loads.len());
        self.loads.insert(index, request);
    }

    /// Ignores the exact invalid ID before any mutation, then cancels a pending
    /// opposite request or appends to the unload FIFO. The early guard is
    /// `0x824ddbe0..0x824ddbe8` in the reference.
    pub fn add_unload(&mut self, request: CollectionRequest) {
        if request.id == INVALID_COLLECTION_ID {
            return;
        }
        debug_assert_eq!(request.kind, CollectionRequestKind::Unload);
        if let Some(index) = self
            .loads
            .iter()
            .position(|pending| pending.id == request.id)
        {
            self.loads.remove(index);
            return;
        }
        self.unloads.push(request);
    }

    /// Updates only the distance of the first pending load from the supplied
    /// focus, matching the reference request updater. Stream order remains
    /// fixed.
    ///
    /// The supplied collection ID is only the invalid-ID guard at
    /// `0x824d7330..0x824d733c`; the scan at `0x824d7368..0x824d737c` compares
    /// `mHandle`, not `mID`.
    pub fn update_load(&mut self, request: &CollectionRequest) {
        debug_assert_eq!(request.kind, CollectionRequestKind::Update);
        if request.id == INVALID_COLLECTION_ID {
            return;
        }
        if let Some(pending) = self
            .loads
            .iter_mut()
            .find(|pending| pending.handle == request.handle)
        {
            pending.distance_xz = request.distance_xz;
        }
    }

    /// Removes the first pending load whose focus distance is the ordered
    /// strict minimum below the reference sentinel.
    ///
    /// This is the selection loop in `cAssetStreamSystem::LoadBestCollection`
    /// (`0x824dbdb8..0x824dbddc`). Equal values retain the earlier request and
    /// unordered comparisons do not replace the winner.
    pub fn take_best_load(&mut self) -> Option<CollectionRequest> {
        let mut best_distance = BEST_LOAD_DISTANCE_SENTINEL;
        let mut best_index = None;

        for (index, request) in self.loads.iter().enumerate() {
            if request.distance_xz < best_distance {
                best_distance = request.distance_xz;
                best_index = Some(index);
            }
        }

        best_index.map(|index| self.loads.remove(index))
    }

    /// Removes the first stream-order-sorted load.
    pub fn take_next_load(&mut self) -> Option<CollectionRequest> {
        (!self.loads.is_empty()).then(|| self.loads.remove(0))
    }

    /// Removes the oldest unload request.
    pub fn take_next_unload(&mut self) -> Option<CollectionRequest> {
        (!self.unloads.is_empty()).then(|| self.unloads.remove(0))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::hash_map::DefaultHasher;

    use super::*;

    fn selected(id: u64, distance_xz: f32) -> FocusCollection {
        FocusCollection {
            index: usize::try_from(id).unwrap(),
            id,
            position: [0.0; 3],
            distance_xz,
            stream_order: i32::try_from(id).unwrap(),
        }
    }

    fn load_request(id: u64, distance_xz: f32, stream_order: i32) -> CollectionRequest {
        CollectionRequest {
            handle: FocusHandle::new(),
            kind: CollectionRequestKind::Load,
            id,
            distance_xz,
            stream_order,
        }
    }

    fn request(
        handle: &FocusHandle,
        kind: CollectionRequestKind,
        id: u64,
        distance_xz: f32,
        stream_order: i32,
    ) -> CollectionRequest {
        CollectionRequest {
            handle: handle.clone(),
            kind,
            id,
            distance_xz,
            stream_order,
        }
    }

    fn handle_hash(handle: &FocusHandle) -> u64 {
        let mut hasher = DefaultHasher::new();
        handle.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn reconciles_membership_in_asset_id_order() {
        let mut focus = StreamFocusState::new([0.0; 3], false);
        let handle = focus.handle().clone();
        let initial = focus.reconcile(&[selected(3, 1.0), selected(1, 2.0)]);
        assert_eq!(initial.current_id, Some(3));
        assert_eq!(
            initial.requests,
            vec![
                CollectionRequest {
                    handle: handle.clone(),
                    kind: CollectionRequestKind::Load,
                    id: 1,
                    distance_xz: 2.0,
                    stream_order: 1,
                },
                CollectionRequest {
                    handle: handle.clone(),
                    kind: CollectionRequestKind::Load,
                    id: 3,
                    distance_xz: 1.0,
                    stream_order: 3,
                },
            ]
        );

        let moved = focus.reconcile(&[selected(2, 0.5), selected(3, 3.0)]);
        assert!(moved.current_changed);
        assert_eq!(moved.current_id, Some(2));
        assert_eq!(
            moved
                .requests
                .iter()
                .map(|request| (request.kind, request.id))
                .collect::<Vec<_>>(),
            vec![
                (CollectionRequestKind::Unload, 1),
                (CollectionRequestKind::Load, 2),
                (CollectionRequestKind::Update, 3),
            ]
        );
        assert!(
            moved
                .requests
                .iter()
                .all(|request| request.handle == handle)
        );
    }

    #[test]
    fn follows_reference_focus_locking_rules() {
        let mut focus = StreamFocusState::new([1.0, 2.0, 3.0], false);
        assert!(!focus.move_to([1.0, 2.0, 3.0], true));
        assert!(!focus.is_locked());
        assert!(focus.move_to([2.0, 2.0, 3.0], true));
        assert!(focus.is_locked());
        assert!(!focus.move_to([3.0, 2.0, 3.0], false));
        assert!(focus.move_to([3.0, 2.0, 3.0], true));
    }

    #[test]
    fn queues_cancel_opposites_and_preserve_reference_ordering() {
        let mut queue = CollectionRequestQueue::default();
        let mut focus = StreamFocusState::new([0.0; 3], false);
        let handle = focus.handle().clone();
        let initial = focus.reconcile(&[selected(30, 3.0), selected(10, 1.0), selected(20, 2.0)]);
        queue.apply_transition(&initial);
        assert_eq!(
            queue
                .pending_loads()
                .iter()
                .map(|request| request.id)
                .collect::<Vec<_>>(),
            vec![10, 20, 30]
        );
        assert_eq!(queue.take_best_load().map(|request| request.id), Some(10));
        assert_eq!(queue.take_next_load().map(|request| request.id), Some(20));

        let moved = focus.reconcile(&[selected(30, 0.5), selected(40, 1.0)]);
        queue.apply_transition(&moved);
        assert_eq!(
            queue
                .pending_loads()
                .iter()
                .map(|request| request.id)
                .collect::<Vec<_>>(),
            vec![30, 40]
        );
        assert_eq!(
            queue
                .pending_unloads()
                .iter()
                .map(|request| request.id)
                .collect::<Vec<_>>(),
            vec![10, 20]
        );
        queue.add_unload(CollectionRequest {
            handle: handle.clone(),
            kind: CollectionRequestKind::Unload,
            id: 40,
            distance_xz: 1.0,
            stream_order: 40,
        });
        queue.add_load(CollectionRequest {
            handle,
            kind: CollectionRequestKind::Load,
            id: 10,
            distance_xz: 2.0,
            stream_order: 10,
        });
        assert_eq!(
            queue
                .pending_loads()
                .iter()
                .map(|request| request.id)
                .collect::<Vec<_>>(),
            vec![30]
        );
        assert_eq!(queue.take_next_unload().map(|request| request.id), Some(20));
    }

    #[test]
    fn focus_handles_clone_by_identity_and_debug_hides_the_host_address() {
        let original = FocusHandle::new();
        let cloned = original.clone();
        let distinct = FocusHandle::new();

        assert_eq!(original, cloned);
        assert_eq!(handle_hash(&original), handle_hash(&cloned));
        assert_ne!(original, distinct);
        assert_eq!(format!("{original:?}"), "FocusHandle");
    }

    #[test]
    fn update_load_uses_the_first_handle_match_not_the_supplied_id() {
        let mut queue = CollectionRequestQueue::default();
        let handle = FocusHandle::new();
        queue.add_load(request(&handle, CollectionRequestKind::Load, 10, 1.0, 10));
        queue.add_load(request(&handle, CollectionRequestKind::Load, 20, 2.0, 20));

        queue.update_load(&request(
            &handle,
            CollectionRequestKind::Update,
            20,
            99.0,
            20,
        ));

        assert_eq!(queue.pending_loads()[0].id, 10);
        assert_eq!(
            queue.pending_loads()[0].distance_xz.to_bits(),
            99.0_f32.to_bits()
        );
        assert_eq!(queue.pending_loads()[1].id, 20);
        assert_eq!(
            queue.pending_loads()[1].distance_xz.to_bits(),
            2.0_f32.to_bits()
        );
    }

    #[test]
    fn update_load_distinguishes_handles_even_for_the_same_id() {
        let mut queue = CollectionRequestQueue::default();
        let first = FocusHandle::new();
        let second = FocusHandle::new();
        queue.add_load(request(&first, CollectionRequestKind::Load, 10, 1.0, 10));
        queue.add_load(request(&second, CollectionRequestKind::Load, 10, 2.0, 10));

        queue.update_load(&request(
            &second,
            CollectionRequestKind::Update,
            10,
            77.0,
            10,
        ));

        assert_eq!(
            queue.pending_loads()[0].distance_xz.to_bits(),
            1.0_f32.to_bits()
        );
        assert_eq!(
            queue.pending_loads()[1].distance_xz.to_bits(),
            77.0_f32.to_bits()
        );
    }

    #[test]
    fn invalid_update_id_is_ignored_before_the_handle_scan() {
        assert_eq!(INVALID_COLLECTION_ID, 0xe171_e1ab_fedb_69ed);
        assert_eq!(
            INVALID_COLLECTION_ID,
            skate2_formats::hash::rw_hash64_string("InvalidStringHash")
        );

        let mut queue = CollectionRequestQueue::default();
        let handle = FocusHandle::new();
        queue.add_load(request(&handle, CollectionRequestKind::Load, 10, 1.0, 10));
        queue.update_load(&request(
            &handle,
            CollectionRequestKind::Update,
            INVALID_COLLECTION_ID,
            99.0,
            10,
        ));

        assert_eq!(
            queue.pending_loads()[0].distance_xz.to_bits(),
            1.0_f32.to_bits()
        );
    }

    #[test]
    fn invalid_load_is_ignored_before_opposite_cancellation() {
        let mut queue = CollectionRequestQueue::default();
        let retained_load = FocusHandle::new();
        let impossible_unload = FocusHandle::new();
        queue.add_load(request(
            &retained_load,
            CollectionRequestKind::Load,
            10,
            1.0,
            10,
        ));
        // The public mutators cannot create this precondition. Seed it through
        // test-only private access to prove the native guard precedes lookup.
        queue.unloads.push(request(
            &impossible_unload,
            CollectionRequestKind::Unload,
            INVALID_COLLECTION_ID,
            2.0,
            20,
        ));
        let before = queue.clone();

        queue.add_load(request(
            &FocusHandle::new(),
            CollectionRequestKind::Load,
            INVALID_COLLECTION_ID,
            3.0,
            30,
        ));

        assert_eq!(queue, before);
    }

    #[test]
    fn invalid_unload_is_ignored_before_opposite_cancellation() {
        let mut queue = CollectionRequestQueue::default();
        let impossible_load = FocusHandle::new();
        let retained_unload = FocusHandle::new();
        // The public mutators cannot create this precondition. Seed it through
        // test-only private access to prove the native guard precedes lookup.
        queue.loads.push(request(
            &impossible_load,
            CollectionRequestKind::Load,
            INVALID_COLLECTION_ID,
            1.0,
            10,
        ));
        queue.add_unload(request(
            &retained_unload,
            CollectionRequestKind::Unload,
            20,
            2.0,
            20,
        ));
        let before = queue.clone();

        queue.add_unload(request(
            &FocusHandle::new(),
            CollectionRequestKind::Unload,
            INVALID_COLLECTION_ID,
            3.0,
            30,
        ));

        assert_eq!(queue, before);
    }

    #[test]
    fn opposite_cancellation_compares_the_full_u64_id() {
        let mut queue = CollectionRequestQueue::default();
        let handle = FocusHandle::new();
        let low_word = 0x5566_7788;
        let first = 0x1111_2222_0000_0000 | low_word;
        let second = 0x9999_aaaa_0000_0000 | low_word;
        queue.add_load(request(
            &handle,
            CollectionRequestKind::Load,
            first,
            1.0,
            10,
        ));
        queue.add_load(request(
            &handle,
            CollectionRequestKind::Load,
            second,
            2.0,
            20,
        ));

        queue.add_unload(request(
            &handle,
            CollectionRequestKind::Unload,
            second,
            2.0,
            0,
        ));

        assert_eq!(queue.pending_loads().len(), 1);
        assert_eq!(queue.pending_loads()[0].id, first);
        assert!(queue.pending_unloads().is_empty());
    }

    #[test]
    fn same_direction_duplicates_and_equal_order_are_stable() {
        let mut queue = CollectionRequestQueue::default();
        let first_handle = FocusHandle::new();
        let second_handle = FocusHandle::new();
        queue.add_load(request(
            &first_handle,
            CollectionRequestKind::Load,
            10,
            1.0,
            50,
        ));
        queue.add_load(request(
            &second_handle,
            CollectionRequestKind::Load,
            10,
            2.0,
            50,
        ));

        assert_eq!(queue.pending_loads().len(), 2);
        assert_eq!(queue.pending_loads()[0].handle, first_handle);
        assert_eq!(queue.pending_loads()[1].handle, second_handle);
    }

    #[test]
    fn best_load_selects_the_ordinary_strict_minimum() {
        let mut queue = CollectionRequestQueue::default();
        queue.add_load(load_request(10, 7.0, 10));
        queue.add_load(load_request(20, -2.0, 20));
        queue.add_load(load_request(30, 3.0, 30));

        assert_eq!(queue.take_best_load().map(|request| request.id), Some(20));
        assert_eq!(
            queue
                .pending_loads()
                .iter()
                .map(|request| request.id)
                .collect::<Vec<_>>(),
            vec![10, 30]
        );
    }

    #[test]
    fn best_load_preserves_the_first_equal_and_signed_zero_candidate() {
        let mut equal = CollectionRequestQueue::default();
        equal.add_load(load_request(10, 4.0, 10));
        equal.add_load(load_request(20, 4.0, 20));
        assert_eq!(equal.take_best_load().map(|request| request.id), Some(10));

        for (first, second) in [(0.0_f32, -0.0_f32), (-0.0_f32, 0.0_f32)] {
            let mut zeros = CollectionRequestQueue::default();
            zeros.add_load(load_request(1, first, 1));
            zeros.add_load(load_request(2, second, 2));
            let winner = zeros.take_best_load().expect("zero is below the sentinel");
            assert_eq!(winner.id, 1);
            assert_eq!(winner.distance_xz.to_bits(), first.to_bits());
        }
    }

    #[test]
    fn best_load_ignores_unordered_nan_candidates() {
        let mut mixed = CollectionRequestQueue::default();
        mixed.add_load(load_request(10, f32::NAN, 10));
        mixed.add_load(load_request(20, 5.0, 20));
        mixed.add_load(load_request(30, f32::from_bits(0xffc0_0001), 30));
        assert_eq!(mixed.take_best_load().map(|request| request.id), Some(20));

        let mut only_nan = CollectionRequestQueue::default();
        only_nan.add_load(load_request(40, f32::NAN, 40));
        assert_eq!(only_nan.take_best_load(), None);
        assert_eq!(only_nan.pending_loads().len(), 1);
    }

    #[test]
    fn best_load_requires_a_candidate_strictly_below_the_exact_sentinel() {
        assert_eq!(BEST_LOAD_DISTANCE_SENTINEL.to_bits(), 0x4e6e_6b28);

        let mut queue = CollectionRequestQueue::default();
        queue.add_load(load_request(10, BEST_LOAD_DISTANCE_SENTINEL, 10));
        queue.add_load(load_request(20, f32::INFINITY, 20));
        queue.add_load(load_request(30, 1_000_000_128.0, 30));

        assert_eq!(queue.take_best_load(), None);
        assert_eq!(queue.pending_loads().len(), 3);
    }
}
