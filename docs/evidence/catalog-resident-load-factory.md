# Catalog-backed resident load factory

## Scope

`catalog_resident_load_factory.rs` binds registered XSM/XST snapshots to the
mounted resident load driver. It supplies a catalog metadata adapter, an
explicit request-ID lease boundary, a per-collection builder-session boundary,
the deferred zero-asset path, and one opaque associated driver type for the
resident streamer.

The factory does not provide a guessed request-ID allocator, resource
allocator, loaded-collection storage, or subsystem registrar. Those remain
explicit production collaborators.

## Metadata and request fields

The asset adapter clones the first-wins `AssetRegistrationSnapshot` from
`WorldStreamCatalog`, preserves the full `u64` ID, and exposes resource-size
lanes 0, 1, and 2 as their raw XST words. The collection pump still owns its
exact 1→0→2 signed-wrapping decision and its separate post-submit reread.
Catalog failure is distinct from an absent metadata record.

Before side effects, `begin` verifies that the full descriptor ID matches the
resident request and that the queued signed asset count has exactly the same
raw 32-bit pattern as the registered XSM count. Stream format is likewise
bit-reinterpreted from registered `u32` to the native raw `i32` domain. Path
and full-width payload offset come directly from the descriptor. XSM payload
size is not invented as a producer bound because the recovered load request
does not pass it at this boundary.

## Zero and mounted paths

Exactly zero creates a deferred driver without constructing a builder session,
allocating a request ID, reading the source, or waking decompression. Its first
driver update marks it complete. Together with the resident streamer's
existing state machine this preserves the three-update trace: create/start,
complete/publish, then allow the next queued request.

Every signed nonzero count, including a negative value whose raw bits match
XSM, takes the mounted path. The factory creates a builder session, obtains an
opaque request-ID lease, rejects exactly `-1`, and retains the lease beside the
mounted driver until the opaque associated driver is dropped. The public
driver is a struct around a private state enum; downstream code cannot extract
the lease, access the inner driver, or clear the first mapped failure.

The public driver exposes a copied read-only mounted-stream snapshot. This
permits a composed corpus gate to prove queued and retained chunks reach zero
without exposing the inner driver, pump, stream, or release authority. The
zero-asset path returns no snapshot because it creates no physical stream.

Request-ID allocation/reuse, lease RAII, builder-session ownership, typed
invalid-domain errors, and session-before-ID ordering are **high** safe-host
policies. Full identities, raw-bit field transfer, exact `-1`, and zero-versus-
signed-nonzero selection are **exact** on the valid path. The composed inner
driver retains each behavior's previously documented confidence.

The factory creates its decompression wake internally and exposes a paired
clone for the resident streamer. Callers must pass that returned clone to the
streamer; the current generic outer constructor does not enforce pairing.
`AtomicBool` synchronization and the driver's serial schedule remain
stronger/different than native concurrent behavior as documented in
[`resident-collection-load-driver.md`](resident-collection-load-driver.md).

## Errors and partial state

Descriptor identity, count bits, builder-session setup, request-ID allocation,
the ended sentinel, and mounted-driver faults remain distinct. A mounted inner
fault is mapped once and the same top-level Arc is replayed thereafter. This
also prevents repeated wrapping from changing error identity.

The resident streamer creates and inserts its empty collection before calling
the factory. Factory failure is therefore retained as the streamer's existing
partial resident state; the factory does not claim transactional rollback of
outer effects.

## Validation and remaining gaps

Eight focused tests cover first-wins XST metadata, high-word IDs, raw resource
words, validation-before-side-effects, zero collaborator suppression and the
three-update outer trace, negative count/high-format bit preservation, valid
negative request IDs, allocator/session failures, exact `-1`, lease lifetime,
producer-construction rollback, final zero queued/retained chunks, and unified
first-fault Arc replay.

The first frozen review rejected a public enum because callers could extract
the lease and clear the mapped-fault cache. The corrected opaque struct/private
state design passed independent review and strict isolated tests.

Production still requires a loaded-collection storage/writer, a concrete
request-ID allocator, a builder-session factory selecting the generic arena
builder, and later per-stream registration/destruction. Exact native allocator
identity and request-ID reuse remain unknown.
