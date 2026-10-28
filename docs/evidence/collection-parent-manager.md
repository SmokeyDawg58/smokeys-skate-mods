# Collection parent manager

## Scope

`collection_parent_manager.rs` reconstructs parent traversal, parent use
counts, registration ordering, ancestor-unload completion buffering, and the
relevant outer manager update sequence. Physical collection ownership remains
in `CollectionResidentStreamer`; Simulation and Presentation delivery remain
result-bearing effects.

The module produces typed request plans because the final application callback
enum and collection asset-count lookup are not wired yet. A `Requested` marker
preserves the original child's caller-supplied callback without guessing its
type.

## Native anchors

| Symbol | Address | Recovered behavior |
|---|---:|---|
| `assman::cAssetStreamManager::LoadCollection` | `0x82d57f58` | recursively queues ancestors before the requested child; full-width sentinel compare at `0x82d57fdc` |
| `assman::cAssetStreamManager::UnloadCollection` | `0x82d58018` | queues requested child before walking ancestors; full-width sentinel compare at `0x82d580ac` |
| `OnLoadParentComplete` | `0x82d5c978` | inserts parent use count one and performs first-use registrations; existing use wraps upward only |
| `NotifyCollectionSimReg` | `0x82d5c468` | resident `UpCollectionLoadedCount` at `0x82d5c490`, then Simulation registration |
| `NotifyCollectionPresReg` | `0x82d5c680` | resident `UpCollectionLoadedCount` at `0x82d5c6ac`, then Presentation registration |
| `cUnloadParentCollectionRequestCallback::OnComplete` | `0x82d5cad0` | appends the completed ancestor's own ID before the resident request's ordinary decrement |
| `UnloadParentComplete` | `0x82d59c60` | decrements parent use, erases at zero, then sends Simulation and Presentation deregistration |
| `cParentCollectionUnloadedCallback::OnComplete` | `0x82d581a0` | later subsystem completion performs one resident decrement |
| `assman::cAssetStreamManager::Update` | `0x82d61318` | resident update, Simulation deferred update, then at most one parent-completion FIFO item |

The runtime initializer at `0x82f760c0` hashes `InvalidStringHash` and installs
`0xe171e1abfedb69ed` as `kInvalidCollectionID`. Only that complete value ends a
parent chain. Zero is an ordinary collection ID.

## Maps, FIFO, and topology

PDB types identify the relation map as
`fixed_hash_map<cAssetID,cAssetID,2048,1024,true>` and the parent-use map as
`fixed_hash_map<cAssetID,uint32_t,16,32,true>`. The final `true` enables
allocator overflow, so 2,048 and 16 are inline capacities rather than proven
total limits. Rust vectors preserve full-width equality and unbounded host
growth without claiming native buckets or allocation failure. Native
`AddParent` overwrites an existing child's parent; the implementation and test
preserve that replacement behavior.

The separate unload-completion vector has a proven non-overflow capacity of
32. A callback at capacity returns a typed safe-host error instead of executing
the native assertion/invalid path. Cycles likewise return a typed error before
any request plan escapes; malformed native cycle behavior is unknown.

The registered BAM corpus has 907 collection records. BAM Presentation has 280
nonterminal records pointing to `cPres_Global`
(`0xe87238efd7f02b45`) and five terminal records; all Simulation and Texture
records are terminal. The 284 Presentation `AddHighCollection` observations
come from the spatial XSS and are not the parent-edge count. The observed graph
has maximum depth one and no cycle. Rob's Park selects six Presentation
children sharing that global parent plus four terminal Simulation and four
terminal Texture collections.

## Counts and effect order

On the first completed parent load, manager use becomes one, active resident
count changes `1 -> 2`, Simulation registration is sent, resident count changes
`2 -> 3`, and Presentation registration is sent. Repeated parent loads are
ordinary resident cache hits followed by a wrapping manager-use increment; no
duplicate registrations occur.

Ancestor unload callbacks append to the manager FIFO before the resident
streamer performs the request's built-in decrement. One resident update may
append several IDs, but the outer parent update consumes only the oldest one.
When a manager use count reaches zero it is erased before Simulation then
Presentation deregistration effects are sent. Their later independent
completion callbacks own the final two resident decrements; this module does
not fabricate immediate completion.

For Rob's Park's six Presentation children, the shared global parent follows
the recovered successful trajectory: first physical resident count one,
first-use registration holds to three, five cache hits produce resident eight
and manager use six, six ancestor unload requests reduce the resident to two
and manager use to zero, and later deregistration completions reduce `2 -> 1 ->
0`.

## Host boundaries and validation

`Rc<RefCell<_>>` plus non-owning `Weak` callbacks is a **high** single-threaded
host representation, not a native allocation/layout claim. Reentrant access,
dropped manager callbacks, FIFO overflow, cycle input, and effect failures are
typed. Observable use-count, FIFO-pop, resident-Up, and successful-effect
mutations are not rolled back after a later failure.

`ParentCollectionCallback::dispatch_mapped` lets a complete application
callback enum delegate either parent variant while retaining its own exact
`ResidentCallbackContext` queue type. The seam exposes neither the queue nor
manager state: load completion can only use the context's active-resident Up,
and unload completion uses only its private weak runtime. The direct callback
implementation delegates through the same path.

Seventeen focused tests cover the exact sentinel and ordinary zero; ancestor-
first/child-first plans; overwrite semantics; full-width hash collisions;
cycle rejection; first-use and repeated-use counts; callback-before-decrement;
same-update FIFO append; maximum-one FIFO consumption; external deregistration
Downs; overflow-enabled maps; exact FIFO capacity; callback trait composition;
outer-enum mapped dispatch; and registration/deregistration failure retention.

The Rob's Park application probe now composes the callback enum and canonical
asset-count/catalog lookup for all emitted plans. Actual
Simulation/Presentation registration systems remain open, so this module is
not a claim that parent collections are live in freeroam.
