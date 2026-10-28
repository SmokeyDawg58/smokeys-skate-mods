//! Typed orchestration for one native streamed-collection load request.
//!
//! This module reconstructs the outer parser, chunk-admission ring, and
//! completion bookkeeping of `assman::tLoadCollectionRequest`. Actual stream
//! I/O, decompression, asset construction, and resident-collection ownership
//! remain explicit collaborators.

use std::{fmt, sync::Arc};

/// Allocated native queue slots. One slot remains empty to distinguish full
/// from empty, so at most 511 entries can be outstanding.
pub const COLLECTION_LOAD_RING_SLOTS: usize = 512;

/// Maximum chunks inspected by one native request update.
pub const COLLECTION_LOAD_CHUNKS_PER_UPDATE: u32 = 16;

/// Destination-resource budget checked after accepting a chunk.
pub const COLLECTION_LOAD_DESTINATION_BUDGET: u32 = 0x0040_0000;

/// Maximum parser stride accepted before native assertion territory.
pub const COLLECTION_LOAD_MAX_STRIDE: u32 = 0x0028_0000;

const COLLECTION_ASSET_HEADER_BYTES: u32 = 24;
const COLLECTION_LOAD_RING_MASK: u32 = 0x1ff;

/// Raw `assman::eStreamFormat` word forwarded without validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CollectionStreamFormat(pub i32);

/// Exact fields consumed from one 24-byte `assman::tCollectionAsset` record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CollectionAssetRecord {
    pub asset_id: u64,
    pub stored_size: u32,
    pub data_offset: u32,
    pub stride: u32,
}

/// One chunk returned by the opaque stream boundary.
#[derive(Debug)]
pub struct CollectionStreamChunk<H> {
    pub request_id: i32,
    pub record: CollectionAssetRecord,
    pub handle: H,
}

/// Native raw-parser result values at `0x82d546b0`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollectionParseDecision {
    MoreDataNeeded,
    Chunk { block_size: u32 },
    ChunkEndRequest { block_size: u32 },
}

/// Stable identity of one callback-ring reservation.
///
/// Native passes a queue-entry pointer. The raw reservation counter is also
/// retained here so a stale host callback cannot alias the same physical slot
/// after 512 later reservations. That additional rejection is a high-
/// confidence safe policy outside the valid native FIFO domain.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CollectionLoadSlot {
    index: u16,
    reservation: u32,
}

impl CollectionLoadSlot {
    #[must_use]
    pub const fn index(self) -> u16 {
        self.index
    }

    #[must_use]
    pub const fn reservation(self) -> u32 {
        self.reservation
    }
}

/// Opaque stream and retained-chunk ownership boundary.
pub trait CollectionChunkStream {
    type Chunk;

    fn is_end_of_stream(&mut self) -> bool;
    fn get_chunk(&mut self) -> Option<CollectionStreamChunk<Self::Chunk>>;
    fn release_chunk(&mut self, chunk: Self::Chunk);

    /// Atomically adds parser stride to the streamer-wide read-byte counter.
    fn account_stream_read_bytes(&mut self, stride: u32);
}

/// Opaque lookup for immutable native `assman::cAssetInfo` records.
pub trait CollectionAssetMetadata {
    type AssetInfo;
    type Error;

    /// Looks up one immutable metadata snapshot without collapsing backend
    /// failure into an absent record.
    ///
    /// # Errors
    ///
    /// Returns the backend's typed lookup or decoding failure.
    fn find_asset_info(&mut self, asset_id: u64) -> Result<Option<Self::AssetInfo>, Self::Error>;

    /// Reads resource-size lanes zero, one, and two in that order-shaped
    /// snapshot. The request deliberately calls this twice on a submitted
    /// asset because the native code rereads the three words for budgeting.
    fn first_three_resource_sizes(&mut self, info: &Self::AssetInfo) -> [u32; 3];
}

/// Borrowed request copied into the native decompression FIFO.
pub struct CollectionDecompressionSubmission<'a, H, I> {
    pub slot: CollectionLoadSlot,
    pub asset_id: u64,
    pub asset_info: &'a I,
    pub chunk: &'a H,
    pub data_offset: u32,
    pub stored_size: u32,
    pub format: CollectionStreamFormat,
}

/// Opaque decompression/asset-construction submission boundary.
pub trait CollectionDecompressionQueue<H, I> {
    type Asset;

    fn submit(&mut self, submission: CollectionDecompressionSubmission<'_, H, I>);
}

/// Opaque ordered `cAssetList::AddAsset` boundary.
pub trait LoadedAssetCollection<A> {
    fn add_asset(&mut self, asset: A);
}

/// Safe host errors for native assertion or memory-unsafe domains.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CollectionLoadPumpError<E> {
    MissingAssetHeader,
    InvalidStride(u32),
    ChunkPresentAtEndOfStream,
    ChunkRequestMismatch {
        expected: i32,
        observed: i32,
    },
    MetadataBackend(E),
    MissingAssetInfo(u64),
    OccupiedQueueSlot(CollectionLoadSlot),
    CompletionOutOfOrder {
        expected: CollectionLoadSlot,
        observed: CollectionLoadSlot,
    },
    EmptyQueueSlot(CollectionLoadSlot),
    RingUnavailable,
    NonEmptyRingAtDrain,
}

impl<E: fmt::Display> fmt::Display for CollectionLoadPumpError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingAssetHeader => formatter.write_str("parser data omitted its asset header"),
            Self::InvalidStride(stride) => {
                write!(
                    formatter,
                    "collection asset stride {stride:#x} is outside 1..={COLLECTION_LOAD_MAX_STRIDE:#x}"
                )
            }
            Self::ChunkPresentAtEndOfStream => {
                formatter.write_str("stream returned a chunk after reporting end-of-stream")
            }
            Self::ChunkRequestMismatch { expected, observed } => write!(
                formatter,
                "stream chunk request ID {observed} does not match active request {expected}"
            ),
            Self::MetadataBackend(source) => {
                write!(formatter, "asset metadata lookup failed: {source}")
            }
            Self::MissingAssetInfo(id) => {
                write!(formatter, "asset {id:#018x} has no metadata record")
            }
            Self::OccupiedQueueSlot(slot) => {
                write!(formatter, "queue slot {} is still occupied", slot.index)
            }
            Self::CompletionOutOfOrder { expected, observed } => write!(
                formatter,
                "completion slot {}/{} does not match FIFO slot {}/{}",
                observed.index, observed.reservation, expected.index, expected.reservation
            ),
            Self::EmptyQueueSlot(slot) => write!(
                formatter,
                "completion slot {}/{} has no retained chunk",
                slot.index, slot.reservation
            ),
            Self::RingUnavailable => formatter.write_str("collection load ring is unavailable"),
            Self::NonEmptyRingAtDrain => {
                formatter.write_str("raw queue counters are equal while a slot remains occupied")
            }
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for CollectionLoadPumpError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::MetadataBackend(source) => Some(source),
            _ => None,
        }
    }
}

pub type CollectionLoadPumpResult<T, E> = Result<T, Arc<CollectionLoadPumpError<E>>>;

/// Observable work performed by one request update.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CollectionLoadUpdate {
    pub chunks_observed: u32,
    pub decompressions_submitted: u32,
    pub chunks_released_without_work: u32,
    pub became_complete: bool,
}

struct PendingChunk<H> {
    reservation: u32,
    chunk: H,
    stored_size: u32,
}

type CollectionLoadRing<H> = Box<[Option<PendingChunk<H>>; COLLECTION_LOAD_RING_SLOTS]>;

/// Typed `assman::tLoadCollectionRequest` parser/ring/update slice.
///
/// Exact anchors are `ParseRawFunction` at `0x82d546b0`, `Load` at
/// `0x82d54bd0`, `EntryComplete` at `0x82d5e4a8`, and `Update` at
/// `0x82d5fc98`. Typed errors and permanent first-fault replay replace native
/// assert/null-dereference behavior and are a high-confidence host policy.
pub struct CollectionLoadPump<S, M, D, C>
where
    S: CollectionChunkStream,
    M: CollectionAssetMetadata,
    D: CollectionDecompressionQueue<S::Chunk, M::AssetInfo>,
    C: LoadedAssetCollection<D::Asset>,
{
    stream: S,
    metadata: M,
    decompression: D,
    collection: C,
    request_id: i32,
    target_assets: i32,
    parsed_assets: i32,
    next_asset: bool,
    pending_stride: u32,
    format: CollectionStreamFormat,
    ring: Option<CollectionLoadRing<S::Chunk>>,
    head: u32,
    tail: u32,
    highest_stored_block: u32,
    loaded_size: u32,
    complete: bool,
    faulted_chunk: Option<CollectionStreamChunk<S::Chunk>>,
    terminal_error: Option<Arc<CollectionLoadPumpError<M::Error>>>,
}

impl<S, M, D, C> CollectionLoadPump<S, M, D, C>
where
    S: CollectionChunkStream,
    M: CollectionAssetMetadata,
    D: CollectionDecompressionQueue<S::Chunk, M::AssetInfo>,
    C: LoadedAssetCollection<D::Asset>,
{
    /// Constructs the state immediately after native `Load`/`QueueFile`.
    #[must_use]
    pub fn new(
        stream: S,
        metadata: M,
        decompression: D,
        collection: C,
        request_id: i32,
        target_assets: i32,
        format: CollectionStreamFormat,
    ) -> Self {
        Self {
            stream,
            metadata,
            decompression,
            collection,
            request_id,
            target_assets,
            parsed_assets: 0,
            next_asset: true,
            pending_stride: 0,
            format,
            ring: Some(Box::new(std::array::from_fn(|_| None))),
            head: 0,
            tail: 0,
            highest_stored_block: 0,
            loaded_size: 0,
            complete: false,
            faulted_chunk: None,
            terminal_error: None,
        }
    }

    #[must_use]
    pub const fn request_id(&self) -> i32 {
        self.request_id
    }

    #[must_use]
    pub const fn parsed_assets(&self) -> i32 {
        self.parsed_assets
    }

    #[must_use]
    pub const fn next_asset(&self) -> bool {
        self.next_asset
    }

    #[must_use]
    pub const fn pending_stride(&self) -> u32 {
        self.pending_stride
    }

    #[must_use]
    pub const fn queue_head(&self) -> u32 {
        self.head
    }

    #[must_use]
    pub const fn queue_tail(&self) -> u32 {
        self.tail
    }

    #[must_use]
    pub const fn highest_stored_block(&self) -> u32 {
        self.highest_stored_block
    }

    #[must_use]
    pub const fn loaded_size(&self) -> u32 {
        self.loaded_size
    }

    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.complete
    }

    #[must_use]
    pub fn is_ring_allocated(&self) -> bool {
        self.ring.is_some()
    }

    #[must_use]
    pub fn terminal_error(&self) -> Option<&Arc<CollectionLoadPumpError<M::Error>>> {
        self.terminal_error.as_ref()
    }

    /// Returns the chunk retained when an unsafe-domain fault occurred after
    /// stream ownership had already transferred to this request.
    #[must_use]
    pub const fn faulted_chunk(&self) -> Option<&CollectionStreamChunk<S::Chunk>> {
        self.faulted_chunk.as_ref()
    }

    #[must_use]
    pub const fn stream(&self) -> &S {
        &self.stream
    }

    #[must_use]
    pub const fn metadata(&self) -> &M {
        &self.metadata
    }

    #[must_use]
    pub const fn decompression(&self) -> &D {
        &self.decompression
    }

    #[must_use]
    pub const fn decompression_mut(&mut self) -> &mut D {
        &mut self.decompression
    }

    #[must_use]
    pub const fn collection(&self) -> &C {
        &self.collection
    }

    /// Applies one native raw-parser callback.
    ///
    /// `record` is intentionally optional: `datasize <= 24` must return before
    /// reading it. Once a stride has been latched, later more-data callbacks
    /// ignore the supplied record until that chunk is emitted.
    ///
    /// # Errors
    ///
    /// Latches an invalid/missing header that enters native assertion or
    /// invalid-memory territory, or replays the first prior fault.
    pub fn parse_raw(
        &mut self,
        datasize: u32,
        record: Option<&CollectionAssetRecord>,
    ) -> CollectionLoadPumpResult<CollectionParseDecision, M::Error> {
        self.check_terminal()?;
        if datasize <= COLLECTION_ASSET_HEADER_BYTES {
            return Ok(CollectionParseDecision::MoreDataNeeded);
        }

        if self.next_asset {
            let Some(record) = record else {
                return self.latch(CollectionLoadPumpError::MissingAssetHeader);
            };
            self.next_asset = false;
            self.pending_stride = record.stride;
            if self.pending_stride == 0 || self.pending_stride > COLLECTION_LOAD_MAX_STRIDE {
                return self.latch(CollectionLoadPumpError::InvalidStride(self.pending_stride));
            }
        }

        if datasize < self.pending_stride {
            return Ok(CollectionParseDecision::MoreDataNeeded);
        }

        let block_size = self.pending_stride;
        self.parsed_assets = self.parsed_assets.wrapping_add(1);
        self.stream.account_stream_read_bytes(block_size);
        if self.parsed_assets >= self.target_assets {
            return Ok(CollectionParseDecision::ChunkEndRequest { block_size });
        }

        self.next_asset = true;
        Ok(CollectionParseDecision::Chunk { block_size })
    }

    /// Performs one native request update.
    ///
    /// # Errors
    ///
    /// Latches and reports a native assertion/unsafe-domain condition, or
    /// replays the first prior fault.
    pub fn update(&mut self) -> CollectionLoadPumpResult<CollectionLoadUpdate, M::Error> {
        self.check_terminal()?;
        let mut result = CollectionLoadUpdate::default();
        if self.complete {
            return Ok(result);
        }

        if self.request_id != -1 {
            if self.stream.is_end_of_stream() {
                if let Some(chunk) = self.stream.get_chunk() {
                    self.faulted_chunk = Some(chunk);
                    return self.latch(CollectionLoadPumpError::ChunkPresentAtEndOfStream);
                }
                self.request_id = -1;
            } else {
                self.pump_chunks(&mut result)?;
            }
        }

        result.became_complete = self.finish_if_drained()?;
        Ok(result)
    }

    /// Completes the exact FIFO ring entry identified by a decompression
    /// callback.
    ///
    /// # Errors
    ///
    /// Latches out-of-order, stale, missing, or post-drain callbacks, or
    /// replays the first prior fault.
    pub fn complete_slot(
        &mut self,
        slot: CollectionLoadSlot,
        asset: Option<D::Asset>,
    ) -> CollectionLoadPumpResult<(), M::Error> {
        self.check_terminal()?;
        let expected = Self::slot_for_counter(self.tail);
        if slot != expected {
            return self.latch(CollectionLoadPumpError::CompletionOutOfOrder {
                expected,
                observed: slot,
            });
        }
        let entry = {
            let Some(ring) = self.ring.as_mut() else {
                return self.latch(CollectionLoadPumpError::RingUnavailable);
            };
            let Some(entry) = ring[usize::from(slot.index)].take() else {
                return self.latch(CollectionLoadPumpError::EmptyQueueSlot(slot));
            };
            entry
        };
        if entry.reservation != slot.reservation {
            let entry_reservation = entry.reservation;
            let Some(ring) = self.ring.as_mut() else {
                return self.latch(CollectionLoadPumpError::RingUnavailable);
            };
            ring[usize::from(slot.index)] = Some(entry);
            return self.latch(CollectionLoadPumpError::CompletionOutOfOrder {
                expected: CollectionLoadSlot {
                    index: slot.index,
                    reservation: entry_reservation,
                },
                observed: slot,
            });
        }

        if let Some(asset) = asset {
            self.loaded_size = self.loaded_size.wrapping_add(entry.stored_size);
            self.collection.add_asset(asset);
        }
        self.stream.release_chunk(entry.chunk);
        self.tail = self.tail.wrapping_add(1);
        Ok(())
    }

    #[must_use]
    pub fn into_parts(self) -> (S, M, D, C) {
        (
            self.stream,
            self.metadata,
            self.decompression,
            self.collection,
        )
    }

    fn pump_chunks(
        &mut self,
        result: &mut CollectionLoadUpdate,
    ) -> CollectionLoadPumpResult<(), M::Error> {
        let mut admitted_count = 0_u32;
        let mut destination_bytes = 0_u32;
        loop {
            if admitted_count >= COLLECTION_LOAD_CHUNKS_PER_UPDATE || self.ring_is_full() {
                break;
            }
            let Some(chunk) = self.stream.get_chunk() else {
                break;
            };
            result.chunks_observed = result.chunks_observed.wrapping_add(1);

            let end_after_chunk = self.stream.is_end_of_stream();

            if chunk.request_id != self.request_id {
                let observed = chunk.request_id;
                self.faulted_chunk = Some(chunk);
                return self.latch(CollectionLoadPumpError::ChunkRequestMismatch {
                    expected: self.request_id,
                    observed,
                });
            }

            let record = chunk.record;
            let asset_info = match self.metadata.find_asset_info(record.asset_id) {
                Ok(Some(asset_info)) => asset_info,
                Ok(None) => {
                    self.faulted_chunk = Some(chunk);
                    return self.latch(CollectionLoadPumpError::MissingAssetInfo(record.asset_id));
                }
                Err(error) => {
                    self.faulted_chunk = Some(chunk);
                    return self.latch(CollectionLoadPumpError::MetadataBackend(error));
                }
            };
            let initial_sizes = self.metadata.first_three_resource_sizes(&asset_info);
            let initial_total = wrapping_first_three(initial_sizes);
            if i32::from_ne_bytes(initial_total.to_ne_bytes()) <= 0 {
                self.stream.release_chunk(chunk.handle);
                result.chunks_released_without_work =
                    result.chunks_released_without_work.wrapping_add(1);
            } else {
                let slot = Self::slot_for_counter(self.head);
                let slot_index = usize::from(slot.index);
                let Some(ring) = self.ring.as_mut() else {
                    self.faulted_chunk = Some(chunk);
                    return self.latch(CollectionLoadPumpError::RingUnavailable);
                };
                if ring[slot_index].is_some() {
                    self.faulted_chunk = Some(chunk);
                    return self.latch(CollectionLoadPumpError::OccupiedQueueSlot(slot));
                }

                self.head = self.head.wrapping_add(1);
                ring[slot_index] = Some(PendingChunk {
                    reservation: slot.reservation,
                    chunk: chunk.handle,
                    stored_size: record.stored_size,
                });
                if record.stored_size > self.highest_stored_block {
                    self.highest_stored_block = record.stored_size;
                }
                let entry = ring[slot_index]
                    .as_ref()
                    .expect("the valid-domain reservation was just installed");
                self.decompression
                    .submit(CollectionDecompressionSubmission {
                        slot,
                        asset_id: record.asset_id,
                        asset_info: &asset_info,
                        chunk: &entry.chunk,
                        data_offset: record.data_offset,
                        stored_size: record.stored_size,
                        format: self.format,
                    });
                result.decompressions_submitted = result.decompressions_submitted.wrapping_add(1);

                let budget_sizes = self.metadata.first_three_resource_sizes(&asset_info);
                destination_bytes =
                    destination_bytes.wrapping_add(wrapping_first_three(budget_sizes));
            }

            if end_after_chunk {
                self.request_id = -1;
                break;
            }
            admitted_count = admitted_count.wrapping_add(1);
            if destination_bytes >= COLLECTION_LOAD_DESTINATION_BUDGET {
                break;
            }
        }
        Ok(())
    }

    fn finish_if_drained(&mut self) -> CollectionLoadPumpResult<bool, M::Error> {
        if self.request_id != -1 || self.tail != self.head {
            return Ok(false);
        }
        let Some(ring) = self.ring.as_ref() else {
            return Ok(false);
        };
        if ring.iter().any(Option::is_some) {
            return self.latch(CollectionLoadPumpError::NonEmptyRingAtDrain);
        }
        self.ring.take();
        self.complete = true;
        Ok(true)
    }

    fn ring_is_full(&self) -> bool {
        ((self.head.wrapping_add(1) ^ self.tail) & COLLECTION_LOAD_RING_MASK) == 0
    }

    fn slot_for_counter(counter: u32) -> CollectionLoadSlot {
        CollectionLoadSlot {
            index: (counter & COLLECTION_LOAD_RING_MASK) as u16,
            reservation: counter,
        }
    }

    fn check_terminal(&self) -> CollectionLoadPumpResult<(), M::Error> {
        match &self.terminal_error {
            Some(error) => Err(Arc::clone(error)),
            None => Ok(()),
        }
    }

    fn latch<T>(
        &mut self,
        error: CollectionLoadPumpError<M::Error>,
    ) -> CollectionLoadPumpResult<T, M::Error> {
        let error = Arc::new(error);
        self.terminal_error = Some(Arc::clone(&error));
        Err(error)
    }
}

fn wrapping_first_three(sizes: [u32; 3]) -> u32 {
    sizes[1].wrapping_add(sizes[0]).wrapping_add(sizes[2])
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, collections::VecDeque, rc::Rc};

    use super::*;

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum Event {
        End(bool),
        Get(u32),
        Account(u32),
        Lookup(u64),
        Sizes(u64, [u32; 3]),
        Submit(CollectionLoadSlot, u64, u32, u32, i32),
        Loaded(u64),
        Release(u32),
    }

    #[derive(Debug)]
    struct TestHandle(u32);

    struct QueuedChunk {
        chunk: CollectionStreamChunk<TestHandle>,
        end_after: bool,
    }

    struct TestStream {
        events: Rc<RefCell<Vec<Event>>>,
        chunks: VecDeque<QueuedChunk>,
        end: bool,
        stream_read_bytes: u64,
    }

    impl CollectionChunkStream for TestStream {
        type Chunk = TestHandle;

        fn is_end_of_stream(&mut self) -> bool {
            self.events.borrow_mut().push(Event::End(self.end));
            self.end
        }

        fn get_chunk(&mut self) -> Option<CollectionStreamChunk<Self::Chunk>> {
            let next = self.chunks.pop_front()?;
            self.events
                .borrow_mut()
                .push(Event::Get(next.chunk.handle.0));
            self.end = next.end_after;
            Some(next.chunk)
        }

        fn release_chunk(&mut self, chunk: Self::Chunk) {
            self.events.borrow_mut().push(Event::Release(chunk.0));
        }

        fn account_stream_read_bytes(&mut self, stride: u32) {
            self.events.borrow_mut().push(Event::Account(stride));
            self.stream_read_bytes = self.stream_read_bytes.wrapping_add(u64::from(stride));
        }
    }

    #[derive(Clone, Debug)]
    struct TestInfo {
        id: u64,
    }

    struct TestMetadata {
        events: Rc<RefCell<Vec<Event>>>,
        missing: Vec<u64>,
        failing: Vec<u64>,
        default_sizes: [u32; 3],
        size_reads: VecDeque<[u32; 3]>,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TestMetadataError(u64);

    impl fmt::Display for TestMetadataError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "injected metadata failure for {:#018x}", self.0)
        }
    }

    impl std::error::Error for TestMetadataError {}

    impl CollectionAssetMetadata for TestMetadata {
        type AssetInfo = TestInfo;
        type Error = TestMetadataError;

        fn find_asset_info(
            &mut self,
            asset_id: u64,
        ) -> Result<Option<Self::AssetInfo>, Self::Error> {
            self.events.borrow_mut().push(Event::Lookup(asset_id));
            if self.failing.contains(&asset_id) {
                return Err(TestMetadataError(asset_id));
            }
            Ok((!self.missing.contains(&asset_id)).then_some(TestInfo { id: asset_id }))
        }

        fn first_three_resource_sizes(&mut self, info: &Self::AssetInfo) -> [u32; 3] {
            let sizes = self.size_reads.pop_front().unwrap_or(self.default_sizes);
            self.events.borrow_mut().push(Event::Sizes(info.id, sizes));
            sizes
        }
    }

    struct TestDecompression {
        events: Rc<RefCell<Vec<Event>>>,
        slots: Vec<CollectionLoadSlot>,
    }

    impl CollectionDecompressionQueue<TestHandle, TestInfo> for TestDecompression {
        type Asset = u64;

        fn submit(
            &mut self,
            submission: CollectionDecompressionSubmission<'_, TestHandle, TestInfo>,
        ) {
            assert_eq!(submission.asset_info.id, submission.asset_id);
            self.events.borrow_mut().push(Event::Submit(
                submission.slot,
                submission.asset_id,
                submission.data_offset,
                submission.stored_size,
                submission.format.0,
            ));
            self.slots.push(submission.slot);
        }
    }

    struct TestCollection {
        events: Rc<RefCell<Vec<Event>>>,
        assets: Vec<u64>,
    }

    impl LoadedAssetCollection<u64> for TestCollection {
        fn add_asset(&mut self, asset: u64) {
            self.events.borrow_mut().push(Event::Loaded(asset));
            self.assets.push(asset);
        }
    }

    type Pump = CollectionLoadPump<TestStream, TestMetadata, TestDecompression, TestCollection>;

    fn record(id: u64, stored_size: u32, stride: u32) -> CollectionAssetRecord {
        CollectionAssetRecord {
            asset_id: id,
            stored_size,
            data_offset: 24,
            stride,
        }
    }

    fn chunk(
        handle: u32,
        request_id: i32,
        record: CollectionAssetRecord,
        end_after: bool,
    ) -> QueuedChunk {
        QueuedChunk {
            chunk: CollectionStreamChunk {
                request_id,
                record,
                handle: TestHandle(handle),
            },
            end_after,
        }
    }

    fn pump(request_id: i32, target_assets: i32, chunks: Vec<QueuedChunk>) -> Pump {
        let events = Rc::new(RefCell::new(Vec::new()));
        CollectionLoadPump::new(
            TestStream {
                events: Rc::clone(&events),
                chunks: chunks.into(),
                end: false,
                stream_read_bytes: 0,
            },
            TestMetadata {
                events: Rc::clone(&events),
                missing: Vec::new(),
                failing: Vec::new(),
                default_sizes: [1, 0, 0],
                size_reads: VecDeque::new(),
            },
            TestDecompression {
                events: Rc::clone(&events),
                slots: Vec::new(),
            },
            TestCollection {
                events,
                assets: Vec::new(),
            },
            request_id,
            target_assets,
            CollectionStreamFormat(3),
        )
    }

    #[test]
    fn parser_short_guard_latch_and_more_data_order_are_exact() {
        let mut pump = pump(7, 2, Vec::new());
        assert_eq!(
            pump.parse_raw(24, None).unwrap(),
            CollectionParseDecision::MoreDataNeeded
        );
        assert!(pump.next_asset());

        let first = record(1, 20, 100);
        assert_eq!(
            pump.parse_raw(25, Some(&first)).unwrap(),
            CollectionParseDecision::MoreDataNeeded
        );
        assert!(!pump.next_asset());
        assert_eq!(pump.pending_stride(), 100);
        let ignored = record(2, 20, 0);
        assert_eq!(
            pump.parse_raw(100, Some(&ignored)).unwrap(),
            CollectionParseDecision::Chunk { block_size: 100 }
        );
        assert!(pump.next_asset());
        assert_eq!(pump.parsed_assets(), 1);
        assert_eq!(pump.stream().stream_read_bytes, 100);
    }

    #[test]
    fn parser_validates_every_stride_including_final_and_replays_first_fault() {
        for bad in [0, COLLECTION_LOAD_MAX_STRIDE + 1] {
            let mut pump = pump(1, 1, Vec::new());
            let header = record(1, 1, bad);
            let first = pump.parse_raw(u32::MAX, Some(&header)).unwrap_err();
            assert!(
                matches!(&*first, CollectionLoadPumpError::InvalidStride(value) if *value == bad)
            );
            let second = pump.parse_raw(u32::MAX, Some(&header)).unwrap_err();
            assert!(Arc::ptr_eq(&first, &second));
            assert_eq!(pump.parsed_assets(), 0);
        }

        let mut maximum = pump(1, 1, Vec::new());
        let header = record(1, 1, COLLECTION_LOAD_MAX_STRIDE);
        assert_eq!(
            maximum.parse_raw(COLLECTION_LOAD_MAX_STRIDE, Some(&header)),
            Ok(CollectionParseDecision::ChunkEndRequest {
                block_size: COLLECTION_LOAD_MAX_STRIDE
            })
        );

        let mut minimum = pump(1, 1, Vec::new());
        let header = record(1, 1, 1);
        assert_eq!(
            minimum.parse_raw(25, Some(&header)),
            Ok(CollectionParseDecision::ChunkEndRequest { block_size: 1 })
        );
    }

    #[test]
    fn parser_uses_signed_target_wrapping_count_and_u64_wrapping_accounting() {
        for target in [0, -1] {
            let mut terminal = pump(1, target, Vec::new());
            let header = record(1, 1, 3);
            assert_eq!(
                terminal.parse_raw(25, Some(&header)).unwrap(),
                CollectionParseDecision::ChunkEndRequest { block_size: 3 }
            );

            let mut pump = pump(1, target, Vec::new());
            pump.stream.stream_read_bytes = u64::MAX - 1;
            pump.parsed_assets = i32::MAX;
            assert_eq!(
                pump.parse_raw(25, Some(&header)).unwrap(),
                CollectionParseDecision::Chunk { block_size: 3 }
            );
            assert_eq!(pump.parsed_assets(), i32::MIN);
            assert_eq!(pump.stream().stream_read_bytes, 1);
        }
    }

    #[test]
    fn update_admits_exactly_sixteen_before_fetching_the_seventeenth() {
        let chunks = (0..17)
            .map(|id| chunk(id, 9, record(u64::from(id), 1, 25), false))
            .collect();
        let mut pump = pump(9, 17, chunks);
        let result = pump.update().unwrap();
        assert_eq!(result.chunks_observed, 16);
        assert_eq!(result.decompressions_submitted, 16);
        assert_eq!(pump.stream().chunks.len(), 1);
        assert_eq!(pump.queue_head(), 16);
    }

    #[test]
    fn decompression_queue_has_explicit_mutable_driver_access() {
        let mut pump = pump(9, 0, Vec::new());
        pump.decompression_mut().slots.push(CollectionLoadSlot {
            index: 3,
            reservation: 515,
        });
        assert_eq!(pump.decompression().slots[0].reservation(), 515);
    }

    #[test]
    fn update_observes_eos_before_metadata_and_rereads_sizes_after_submit() {
        let mut pump = pump(9, 1, vec![chunk(7, 9, record(11, 13, 25), false)]);
        pump.update().unwrap();
        assert_eq!(
            &*pump.stream().events.borrow(),
            &[
                Event::End(false),
                Event::Get(7),
                Event::End(false),
                Event::Lookup(11),
                Event::Sizes(11, [1, 0, 0]),
                Event::Submit(
                    CollectionLoadSlot {
                        index: 0,
                        reservation: 0
                    },
                    11,
                    24,
                    13,
                    3
                ),
                Event::Sizes(11, [1, 0, 0]),
            ]
        );
    }

    #[test]
    fn destination_budget_is_reread_and_checked_only_after_accepted_chunk() {
        let chunks = vec![
            chunk(1, 5, record(1, 10, 25), false),
            chunk(2, 5, record(2, 20, 25), false),
        ];
        let mut pump = pump(5, 2, chunks);
        pump.metadata.size_reads =
            VecDeque::from([[1, 0, 0], [COLLECTION_LOAD_DESTINATION_BUDGET, 0, 0]]);
        let result = pump.update().unwrap();
        assert_eq!(result.chunks_observed, 1);
        assert_eq!(result.decompressions_submitted, 1);
        assert_eq!(pump.stream().chunks.len(), 1);
        assert_eq!(
            pump.metadata()
                .events
                .borrow()
                .iter()
                .filter(|event| matches!(event, Event::Sizes(..)))
                .count(),
            2
        );
    }

    #[test]
    fn first_chunk_over_budget_is_accepted_and_exactly_equal_stops_next_fetch() {
        for budget in [
            COLLECTION_LOAD_DESTINATION_BUDGET,
            COLLECTION_LOAD_DESTINATION_BUDGET + 1,
        ] {
            let chunks = vec![
                chunk(1, 5, record(1, 1, 25), false),
                chunk(2, 5, record(2, 1, 25), false),
            ];
            let mut pump = pump(5, 2, chunks);
            pump.metadata.default_sizes = [budget, 0, 0];
            let result = pump.update().unwrap();
            assert_eq!(result.decompressions_submitted, 1);
            assert_eq!(pump.stream().chunks.len(), 1);
        }
    }

    #[test]
    fn signed_nonpositive_resource_sum_releases_without_ring_or_budget() {
        let chunks = vec![
            chunk(1, 5, record(1, 7, 25), false),
            chunk(2, 5, record(2, 8, 25), false),
            chunk(3, 5, record(3, 9, 25), false),
        ];
        let mut pump = pump(5, 3, chunks);
        pump.metadata.size_reads = VecDeque::from([
            [u32::MAX, 1, 0],
            [0x8000_0000, 0, 0],
            [u32::MAX, 2, 0],
            [COLLECTION_LOAD_DESTINATION_BUDGET, 0, 0],
        ]);
        let result = pump.update().unwrap();
        assert_eq!(result.chunks_released_without_work, 2);
        assert_eq!(result.decompressions_submitted, 1);
        assert_eq!(pump.queue_head(), 1);
        assert_eq!(pump.highest_stored_block(), 9);
    }

    #[test]
    fn ring_has_512_stable_slots_but_full_at_511_outstanding() {
        let chunks = (0..512)
            .map(|id| chunk(id, 6, record(u64::from(id), 1, 25), false))
            .collect();
        let mut pump = pump(6, 512, chunks);
        for _ in 0..32 {
            pump.update().unwrap();
        }
        assert_eq!(pump.queue_head(), 511);
        assert_eq!(pump.queue_tail(), 0);
        assert!(pump.ring_is_full());
        assert_eq!(pump.stream().chunks.len(), 1);
        assert_eq!(pump.update().unwrap().chunks_observed, 0);
    }

    #[test]
    fn full_formula_and_slot_index_use_low_nine_bits_with_wrapping_counters() {
        let mut pump = pump(1, 1, Vec::new());
        pump.head = u32::MAX;
        pump.tail = 0;
        assert!(pump.ring_is_full());
        assert_eq!(Pump::slot_for_counter(u32::MAX).index(), 511);
        pump.tail = 1;
        assert!(!pump.ring_is_full());
        pump.head = 512;
        pump.tail = 1;
        assert!(pump.ring_is_full());
    }

    #[test]
    fn only_exact_minus_one_is_the_ended_request_sentinel() {
        let mut pump = pump(-2, 0, Vec::new());
        pump.update().unwrap();
        assert_eq!(pump.request_id(), -2);
        assert!(!pump.is_complete());
        assert_eq!(&*pump.stream().events.borrow(), &[Event::End(false)]);
    }

    #[test]
    fn end_after_chunk_sets_minus_one_before_budget_and_drains_later() {
        let chunks = vec![chunk(7, 42, record(9, 15, 25), true)];
        let mut pump = pump(42, 1, chunks);
        let result = pump.update().unwrap();
        assert_eq!(result.decompressions_submitted, 1);
        assert_eq!(pump.request_id(), -1);
        assert!(!pump.is_complete());
        let slot = pump.decompression().slots[0];
        pump.complete_slot(slot, Some(99)).unwrap();
        let result = pump.update().unwrap();
        assert!(result.became_complete);
        assert!(pump.is_complete());
        assert!(!pump.is_ring_allocated());
    }

    #[test]
    fn null_get_retries_but_chunk_after_entry_eos_latches() {
        let mut retry = pump(4, 1, Vec::new());
        assert_eq!(retry.update().unwrap().chunks_observed, 0);
        assert_eq!(retry.request_id(), 4);

        let mut bad = pump(4, 1, vec![chunk(1, 4, record(1, 1, 25), false)]);
        bad.stream.end = true;
        let first = bad.update().unwrap_err();
        assert_eq!(&*first, &CollectionLoadPumpError::ChunkPresentAtEndOfStream);
        let second = bad.update().unwrap_err();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(bad.request_id(), 4);
    }

    #[test]
    fn mismatched_request_and_metadata_faults_are_terminal_without_release() {
        let mut mismatch = pump(8, 1, vec![chunk(1, 9, record(1, 1, 25), false)]);
        assert!(matches!(
            &*mismatch.update().unwrap_err(),
            CollectionLoadPumpError::ChunkRequestMismatch {
                expected: 8,
                observed: 9
            }
        ));
        assert!(
            !mismatch
                .stream()
                .events
                .borrow()
                .iter()
                .any(|event| matches!(event, Event::Release(_)))
        );

        let mut missing = pump(8, 1, vec![chunk(1, 8, record(77, 1, 25), false)]);
        missing.metadata.missing.push(77);
        assert_eq!(
            &*missing.update().unwrap_err(),
            &CollectionLoadPumpError::MissingAssetInfo(77)
        );

        let mut failed = pump(8, 1, vec![chunk(2, 8, record(88, 1, 25), false)]);
        failed.metadata.failing.push(88);
        let first = failed.update().unwrap_err();
        assert_eq!(
            &*first,
            &CollectionLoadPumpError::MetadataBackend(TestMetadataError(88))
        );
        let second = failed.update().unwrap_err();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(failed.faulted_chunk().unwrap().handle.0, 2);
        assert!(
            !failed
                .stream()
                .events
                .borrow()
                .iter()
                .any(|event| matches!(event, Event::Release(_)))
        );
    }

    #[test]
    fn successful_completion_orders_loaded_bytes_add_release_then_tail() {
        let mut pump = pump(3, 1, vec![chunk(11, 3, record(5, 23, 25), false)]);
        pump.update().unwrap();
        let slot = pump.decompression().slots[0];
        pump.stream.events.borrow_mut().clear();
        pump.complete_slot(slot, Some(5)).unwrap();
        assert_eq!(pump.loaded_size(), 23);
        assert_eq!(pump.queue_tail(), 1);
        assert_eq!(
            &*pump.stream().events.borrow(),
            &[Event::Loaded(5), Event::Release(11)]
        );
    }

    #[test]
    fn stored_size_accounting_wraps_and_repeated_asset_occurrences_are_not_deduped() {
        let mut pump = pump(
            3,
            2,
            vec![
                chunk(11, 3, record(5, 10, 25), false),
                chunk(12, 3, record(5, 10, 25), false),
            ],
        );
        pump.loaded_size = u32::MAX - 5;
        pump.update().unwrap();
        let slots = pump.decompression().slots.clone();
        assert_eq!(slots.len(), 2);
        assert_eq!(pump.highest_stored_block(), 10);
        pump.complete_slot(slots[0], Some(5)).unwrap();
        pump.complete_slot(slots[1], Some(5)).unwrap();
        assert_eq!(pump.loaded_size(), 14);
        assert_eq!(pump.collection().assets, [5, 5]);
    }

    #[test]
    fn null_asset_completion_releases_and_advances_without_accounting_or_add() {
        let mut pump = pump(3, 1, vec![chunk(11, 3, record(5, 23, 25), false)]);
        pump.update().unwrap();
        let slot = pump.decompression().slots[0];
        pump.stream.events.borrow_mut().clear();
        pump.complete_slot(slot, None).unwrap();
        assert_eq!(pump.loaded_size(), 0);
        assert!(pump.collection().assets.is_empty());
        assert_eq!(&*pump.stream().events.borrow(), &[Event::Release(11)]);
        assert_eq!(pump.queue_tail(), 1);
    }

    #[test]
    fn completion_requires_exact_fifo_slot_and_replays_fault() {
        let mut pump = pump(
            3,
            2,
            vec![
                chunk(1, 3, record(1, 1, 25), false),
                chunk(2, 3, record(2, 1, 25), false),
            ],
        );
        pump.update().unwrap();
        let observed = pump.decompression().slots[1];
        let first = pump.complete_slot(observed, Some(2)).unwrap_err();
        assert!(matches!(
            &*first,
            CollectionLoadPumpError::CompletionOutOfOrder { expected, observed: got }
                if expected.index() == 0 && *got == observed
        ));
        let second = pump
            .complete_slot(pump.decompression().slots[0], Some(1))
            .unwrap_err();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(pump.queue_tail(), 0);
        let ring = pump.ring.as_ref().unwrap();
        assert!(ring[0].is_some());
        assert!(ring[1].is_some());
        assert!(pump.collection().assets.is_empty());
    }

    #[test]
    fn completion_requires_raw_counter_equality_not_only_masked_equality() {
        let mut pump = pump(-1, 0, Vec::new());
        pump.head = 512;
        pump.tail = 0;
        assert!(!pump.update().unwrap().became_complete);
        assert!(!pump.is_complete());
    }

    #[test]
    fn stale_same_index_completion_is_rejected_by_raw_reservation_without_mutation() {
        let mut pump = pump(3, 1, vec![chunk(1, 3, record(1, 1, 25), false)]);
        pump.head = 512;
        pump.tail = 512;
        pump.update().unwrap();
        assert_eq!(pump.decompression().slots[0].index(), 0);
        assert_eq!(pump.decompression().slots[0].reservation(), 512);

        let stale = CollectionLoadSlot {
            index: 0,
            reservation: 0,
        };
        let fault = pump.complete_slot(stale, Some(1)).unwrap_err();
        assert!(matches!(
            &*fault,
            CollectionLoadPumpError::CompletionOutOfOrder { expected, observed }
                if expected.reservation() == 512 && *observed == stale
        ));
        assert_eq!(pump.queue_tail(), 512);
        assert!(pump.ring.as_ref().unwrap()[0].is_some());
        assert!(pump.collection().assets.is_empty());
    }

    #[test]
    fn fifo_completion_tail_uses_wrapping_raw_arithmetic() {
        let mut pump = pump(-1, 0, Vec::new());
        pump.tail = u32::MAX;
        pump.head = 0;
        pump.ring.as_mut().unwrap()[511] = Some(PendingChunk {
            reservation: u32::MAX,
            chunk: TestHandle(77),
            stored_size: 9,
        });

        pump.complete_slot(Pump::slot_for_counter(u32::MAX), None)
            .unwrap();
        assert_eq!(pump.queue_tail(), 0);
        assert_eq!(&*pump.stream().events.borrow(), &[Event::Release(77)]);
    }

    #[test]
    fn zero_asset_minus_one_request_frees_empty_ring_on_first_update() {
        let mut pump = pump(-1, 0, Vec::new());
        let result = pump.update().unwrap();
        assert!(result.became_complete);
        assert!(pump.is_complete());
        assert!(!pump.is_ring_allocated());
        assert!(pump.stream().events.borrow().is_empty());
    }
}
