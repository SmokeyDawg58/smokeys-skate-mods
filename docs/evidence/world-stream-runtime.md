# Concrete world-stream runtime publisher

This record covers the concrete `LoadedStreamPublisher` that composes the
parsed registration catalog, file-local collision Octree, processed XSS
storage, and global publication store. It closes the ownership bridge between
the already reconstructed components without adding an emulator or a second
container model.

## Evidence anchors

| Address | Behavior |
|---:|---|
| `0x824e0ba8` | `cStreamFile::Update` |
| `0x824e0d64..0x824e1120` | source-order LOD and already-valid filtering, metadata construction |
| `0x824e1124..0x824e12f8` | compact four-lane bounds and 200-unit expansion |
| `0x824e12fc..0x824e136c` | local Octree descriptor, allocation, Initialize, and ownership store |
| `0x824e1398..0x824e13c0` | ascending compact local insertion |
| `0x824e1ad0` | global loaded-file publication |
| `0x824e1420..0x824e1468` | ascending local removal and release/free |
| `0x824e1d2c..0x824e1d50` | raw-6 gate, global removal, and file destruction |
| `0x82adbee8` | `Octree::GetResourceDescriptor` |
| `0x82ae0760` | `Octree::Initialize` |
| `0x82ad6af0` | `Octree::Insert` |
| `0x82acc210` | `Octree::Remove` |
| `0x825187c0` | empty `Octree::Release` |

## Ownership and authority

`WorldStreamRuntime` retains the exact `RegistrationFileId` from the callback
context. It never reconstructs identity from kind, path, or stamp. The shared
`WorldStreamCatalog` is the sole authority for XSM payload offset and size.
The `PublishedStreamStore` is the sole authority for
`is_collection_valid`; parsed XSM membership does not imply that an XSS record
is globally published.

The adapter receives a unique, non-cloneable local builder. Capacity and the
five-slot resource plan use the raw XSS logical length, including zero and the
exact `u16` maximum of 65,535. Only the initialized accepted prefix is inserted.
Each insert must arrive at its ascending compact index and forwards all four
raw bbox lanes. Publication consumes the builder and seals it only after
checking kind, raw length, accepted length, stream type, compact bbox index, and
bit-exact bbox words against `ProcessedOctreeStorage`.

The sealed local owner moves by value into the global store. Compact IDs are
published in accepted-prefix order, preserving the store's first-ID-wins rule
and stable per-kind file ordering. Neither the adapter nor caller can retain a
strong alias to the local Octree. The public associated builder, owner, and
token names exist only because the public trait implementation requires them;
their fields and constructors are private and none is cloneable.

## Release boundary

Destroying the store-owned local resource calls `remove_next` exactly once for
each accepted compact index in ascending order, then destroys the Octree
storage. The adapter waits for this synchronous release during raw state 5.
Only a successful report permits the later global unpublish call after raw
state 6.

Native valid-domain removal has no error return. Rust records a checked removal
failure, poison, or incomplete destruction through a private shared report and
permanently refuses runtime unpublication for that token. A destructor panic is
caught by the global store and likewise cannot expose global removal. The token
has no automatic-unpublish destructor, so dropping an outer owner cannot invent
state ordering.

## Real BAM composition probe

The application `--probe-world-stream` path now composes the real mount-backed
reader, parsed catalog, adapter, local Octree owner, global publication store,
live Texture gate, and outer `WorldStreamLoads` state machine against the
supplied `515600` debug corpus. It validates the exact compressed outer trace
`[1, 2, 3, 4, 5, 6, 7, 0]`, then performs one additional dormant poll to prove
that the case-seven completion indication is a one-poll pulse before moving the
completed record into its diagnostic owner.

The retained Presentation, Simulation, and Texture files match their exact BAM
paths, version-3/stream-format-2 manifests, stamps, and corpus counts. The
catalog owns 907 XSM collections, 23,667 XST references and 23,667 unique XST
asset IDs, three XSS tables, and two XML files. The XSS tables contain 906
distinct accepted IDs in total: Presentation 284, Simulation 284, and Texture
338. Presentation XML retains 175,633 bytes, Simulation XML 172,291 bytes, and
Texture correctly has no XML.

One shared observation sequence proves 284 ordered Presentation
`AddHighCollection(id, x, z)` calls, followed by the single enabled Texture-gate
sample, followed by the single final-bounds call. Every ID and signed coordinate
word is checked against the processed Presentation record using the recovered
`floor(position + 0.5)` conversion. Publication exposes exactly one file per
kind and the same 906 authored XSS IDs.

The diagnostic owner then requests unload in Texture, Simulation, Presentation
order. Each file takes exactly three polls with raw states `[5, 5, 6]`; the
probe checks the catalog and publication store after every poll, including the
XSM/XST removal on poll two and local-release/global-unpublication boundary on
poll three. The final catalog and global store are empty.

This probe proves manifest loading and local/global publication ownership for
the supplied corpus. It does **not** load an XSF collection, construct Rob's
Park cells, dispatch arena resources, or reproduce the production
`LoadController` schedule.

## External effects and confidence

The texture-stream decision delegates to a live gate every time the adapter
samples it. Immediate Presentation `AddHighCollection` and final world-bounds
effects delegate to explicit collaborators at the already recovered adapter
points. Their unrecovered internals are not replaced with generic behavior.

Valid-domain authority, capacities, raw/compact separation, insertion and
removal order, bbox forwarding, identity, and local-before-global lifetime are
**exact**. Rust typestate, private non-cloneable capabilities, checked joins,
failure latching, and shared reports are **high-confidence safe policies**.
Exceptional PPC/VMX float parity, allocator exhaustion, and the concrete
effect/gate owners remain unknown or external boundaries.

Thirteen focused tests cover capacities 0, 284, 285, 65,535 and rejection of
65,536; zero/default and raw-greater-than-accepted cases; exact metadata and
bbox forwarding; seal mismatches; catalog/store authority; stable identity and
collisions; normal and failed removal; one-shot tokens; and live effects. Two
independent frozen reviews passed, including strict temporary-export and full
workspace validation.
