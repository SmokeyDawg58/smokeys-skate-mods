# World stream format evidence

Last updated: 2026-09-03

## Proven native data path

The reconstructed path from installed game data to individual world objects is:

```text
BIG/Viv4 mount
  -> BAM_Pres or BAM_Sim manifest
     -> XSM collection table (CMAP)
     -> XST asset table (ATOC)
     -> XSS spatial cells (CSPA)
        -> selected XSF collection (SFIL)
           -> collection asset block
              -> five-segment compressed arena
                 -> RW4 resource classes
                    -> arena object dictionary
```

Every layer in this chain is parsed by checked, human-readable Rust. No Xbox
emulation or translated PPC instructions are involved.

## Debug-type contracts

The following record sizes and field layouts come directly from named debug
types in the reference database:

| Debug type | Bytes | Rust use |
|---|---:|---|
| `assman::tAssetMapHeader` | 24 | XST header |
| `assman::tAssetMapEntry` | 64 | XST asset record |
| `assman::tCollectionMapHeader` | 32 | XSM header |
| `assman::tCollectionMapEntry` | 104 | XSM collection record |
| `assman::tCollectionSpaceHeader` | 24 | XSS header |
| `assman::tCollectionSpaceEntry` | 80 | XSS bounds record |
| `assman::tCollectionAsset` | 24 | XSF asset block |
| `assman::tCompressedArenaHeader` | 80 | five source/destination segment descriptors |
| `rw::core::arena::ArenaFileHeaderMagicNumber` | 12 | RW4 magic |
| `rw::core::arena::ArenaFileHeader` | 28 | endian, pointer, and version metadata |
| `rw::core::arena::Arena` | 172 | resource-zero arena header |
| `rw::core::arena::ArenaDictEntry` | 24 | serialized object record |

The RW4 dictionary contains an object offset, relocation offset, byte size,
alignment, type-table index, and type ID. Base-resource IDs `00010030` through
`00010033` select resource classes 1 through 4; ordinary typed objects reside
in class 0. The native parser validates all five resource sizes and every object
range against its selected class before exposing bytes to a format-specific
decoder.

## Inner asynchronous manifest handshake

The checked Rust slice in `crates/skate2-core/src/streaming/game_stream.rs` reconstructs
the inner `cStreamFile`/`cGameStream` load-state boundary used beneath the
outer BAM handshake. It intentionally stops at typed asynchronous reads and
registration callbacks: the console file loader, XSF loading, asset-manager
internals, and scheduling are not inferred.

### Symbols and layouts

The function addresses are exact for the supplied reference executable:

| Address | Symbol | Recovered role |
|---:|---|---|
| `0x824d7748` | `cStreamFile::Load` | build paths, allocate/start nested stream, store ID, enter raw 1 |
| `0x824d7838` | `cStreamFile::Unload` | request nested unload and enter raw 5 |
| `0x824d7c60` | `cStreamFile::LoadStreamXml` | begin the separate XML read |
| `0x824d8ef8` | `cStreamFile::cStreamFile` | initialize file and XML raw states to 0 |
| `0x824d9bf0` | `cAssetStreamSystem::LoadStreamFile` | own/create the `cStreamFile` record |
| `0x824e0ba8` | `cStreamFile::Update` | one file-state dispatch followed by one XML-state dispatch |
| `0x824e1ad0` | `cAssetStreamSystem::UpdateStreamFileLoaded` | report loaded only when file raw state is 4 |
| `0x82aba3f0` | `AssetPaths::BuildStreamPath` | derive collection directory and typed manifest stem |
| `0x82aba478` | `AssetPaths::GetStreamTypeSuffix` | select `Pres`, `Sim`, or `Tex` |
| `0x82d540f0` | `cGameStream::Unload` | set the nested unload-request byte |
| `0x82d542c8` | `cGameStream::Load` | retain paths/type/ID and enter observed raw 1 |
| `0x82d5db60` | `cGameStream::Update` | one nested-state dispatch |

PDB layouts are also exact. `cGameStream` is `0x260` bytes: manager and loader
pointers occupy `+0x08..+0x20`, raw state is at `+0x24`, the 256-byte stream
path and filename buffers start at `+0x28` and `+0x128`, the unload,
collections-loaded, and asset-info-loaded bytes are at `+0x228..+0x22a`, and
the map stamp/count/vectors occupy `+0x230..+0x25f`. `cStreamFile` is `0x290`
bytes: its raw file and XML states are at `+0x28` and `+0x2c`, stream type is
at `+0x40`, async operation at `+0x44`, local Octree pointer at `+0x48`, nested
stream ID at `+0x50`, raw XSS/Octree entry count at `+0x58`, retained file and
XML path buffers at `+0x5c` and `+0x15c`, accepted spatial-record count at
`+0x25c`, the `tOctreeInfo` vector at `+0x260`, and the bounding box at
`+0x270`. PDB `tOctreeInfo` is `0x80` bytes.

The PDB names these `eStreamFileState` values exactly: Init 0,
LoadStreamWait 1, LoadSpaceFileWait 2, LoadStreamXmlWait 3, LoadDone 4,
UnloadWait 5, and UnloadDone 6. It names nested `eGameStreamState` values
LoadingMap 2, LoadingTOC 3, Loaded 4, Unloading 6, Unloaded 7, and LoadFailed
8. Nested raw 1 and raw 5 are observed assignments/transitions but have no PDB
enumerator name, so the Rust names them descriptively. In particular,
`cGameStream` raw 8 is not a `cStreamFile` state. The XML enum is Init 0,
LoadStreamXmlWait 1, and LoadDone 2. All three Rust wrappers preserve arbitrary
raw `u32` values; impossible values produce invariant errors when polled.

### Exact load and failure order

For a base path `data\content\world\stream`, basename `BAM`, and presentation
stream, `BuildStreamPath` produces collection directory
`data\content\world\stream\BAM\` and manifest stem
`data\content\world\stream\BAM\BAM_Pres`. It inserts the shown separators but
otherwise preserves the caller's slash bytes, including forward slashes and
duplicate separators. The native fixed buffers are 256 bytes; the Rust
derivation rejects interior NULs and any derived C string longer than 255 bytes
instead of reproducing an overflow. Tests lock both the accepted 255-byte and
rejected 256-byte boundaries.

One update dispatch follows this exact ordering:

1. Nested observed raw 1 starts `<stem>.xsm` with loader flag 1 and alignment
   `0x10`, then stores raw 2.
2. Nested raw 2 polls that read. Pending stays raw 2. Read failure releases the
   operation and stores raw 8. Completion releases it; a late unload request
   discards the bytes and stores observed raw 5. Otherwise the CMAP is consumed,
   collection registration runs, and `<stem>.xst` begins in the same poll with
   flag 1/alignment `0x10`, followed by raw 3.
3. Nested raw 3 reacts only to completion. Pending and read failure both retain
   raw 3 and the operation. A completed ATOC registers asset information and
   stores raw 4, unless the late unload branch discards it and stores raw 5.
4. Enclosing file raw 1 pumps the nested manager. If nested raw 4 is visible
   after that pump, it begins `<stem>.xss` in the same poll with flag 0 and
   alignment `0x10`, then stores file raw 2.
5. File raw 2 reacts only to XSS completion. Pending and read failure retain
   raw 2 and the operation. Completion consumes CSPA, finalizes the spatial
   structures/bounds through the collaborator, and stores file raw 4.

Thus the executable load order is `.xsm -> .xst -> .xss`; this differs from a
pure format dependency listing. The implementation reuses `CollectionMap`,
`AssetToc`, and `CollectionSpace` and imposes no 16-record manifest cap. A
focused 17-collection test guards that distinction from the outer controller's
separate fixed stream-file-record capacity.

The separate XML operation begins `data\stream\<basename>_<suffix>.xml` with
flag 0/alignment `0x10`. `LoadStreamXml` publishes XML raw 1 immediately before
the `ASyncLoad` call; the safe Rust boundary preserves that call-time ordering
but rolls back to raw 0 if its collaborator cannot return an initialized owned
handle. The outer controller invokes it for Presentation and Simulation only;
Texture does not pass this stage. XML pending remains raw 1,
completion runs the opaque XML collaborator and stores raw 2, while XML read
failure also clears the operation and stores raw 2 without invoking the XML
collaborator. `cStreamFile::Update` dispatches XML after its main raw-state
case. The public safe API requires file raw 4 before beginning XML, matching
the proven outer call order and preventing the single file-level read slot from
being occupied before XSS starts. It also rejects Texture at this outer-policy
boundary; `cStreamFile::LoadStreamXml` itself contains neither guard. The Rust
API keeps the XML contents opaque because no evidence-backed XML schema is
reconstructed here.

Nested unload is limited to the observed mechanics. The unload request is a
separate byte. Loaded or LoadFailed transitions to observed raw 5 on a later
update; raw 5 removes retained collection then asset registrations and stores
raw 6; raw 6 stores raw 7 on the next update. File raw 5 pumps that nested
machine and performs spatial/XML cleanup when raw 7 becomes visible, storing
file raw 6. The safe API exposes only file-unload entry states whose pending-I/O
ownership is established (load-wait and loaded); it reports other entry states
as unsupported rather than guessing cancellation behavior.

The main DMO bank exposes only `DMO_Pres.xsm` and `DMO_Pres.xst` at this
manifest boundary; its hull-loader path does not require the ordinary world
stream's XSS or XML stages. `ManifestOnlyStream` is therefore a deliberately
named **host composition** around the same recovered `GameStream`. It owns a
fresh registration-file identity, drives the exact asynchronous XSM then XST
states and callbacks, stops at nested raw 4, and exposes neither an XSS nor XML
operation. On a clean explicit unload, it drives the unchanged nested raw 5,
6, and 7 states, then calls the existing file-owner removal callback with
`(None, false)` once. The wrapper permanently latches the first nested or
final-removal error and does no implicit `Drop` cleanup. An XST raw-status-1
stall is not unloadable through the recovered nested state machine. Parser,
callback, or partial-unload faults deliberately retain whichever catalog
prefix or suffix was already applied because no rollback/retry authority is
proven. The successful path closes the normal ownership gap for
catalog-registering manifest-only banks; it is not evidence for a second
native `cStreamFile` class or for hull-loader scheduling.

Malformed CMAP/ATOC/CSPA, unexpected CMAP/ATOC version, absent or mismatched
async handles, and invalid raw states are explicit errors. The XSS path does
not add a version-3 gate: `cStreamFile::Update` consumes its header/records
without the version assertion seen in the nested XSM/XST cases, and a focused
test retains an unusual parsed CSPA version. Errors are not mapped to Pending
or to native raw 8: executable evidence ties raw 8 specifically to the XSM
async-read failure branch. Poll-time parser, invariant, begin-read, callback,
and partial-unload errors are stored as the first shared terminal error; later
polls return that same error allocation and perform no repeated side effects.
Native status-1 read failures remain distinct: XSM enters raw 8, XST stays raw
3 with its operation, and XSS stays file raw 2 with its operation. A retained
observation exposes all three to a safe adapter without rewriting those exact
raw states. The async backend contract requires status 1 to remain sticky for
a retained handle. XML status 1 remains ordinary completion and is deliberately
absent from the failure observation. The backend callback groups native
file-local construction work at the narrowest presently justified boundary;
exact per-record asset-manager internals remain outside this slice.

Confidence is **exact** for symbols, addresses, layouts, raw transitions,
branch distinctions, suffixes, input-slash preservation, read order, flags,
alignment, same-poll starts, and the asymmetric XSM/XST/XSS/XML failure
behavior. It is **high** for safe owned path validation, terminal-error
latching, failure observation, and explicit invariant errors because those
preserve valid-input behavior while refusing native memory-unsafe or
infinite-wait cases. The FileLoader pool, admission, release, and worker/load-
thread split are now concrete in `async-stream-loader.md`. Device-level seek
ordering, XML schema semantics, and cancellation during an outstanding XSS/XML
operation remain unknown and are not implemented. XMM is diagnostic corpus
evidence only and is not a runtime dependency of this state machine.

The exported `world_stream_adapter` now stores a
`game_stream::StreamFile<Handle, Error>` in each result-bearing outer backend
slot and translates the three stream kinds directly. Parser, ownership,
publication, and safe invariant failures latch their first shared cause and do
not become ordinary Pending or repeat already-applied side effects. Native
XSM/XST/XSS failure stalls remain distinct observations rather than invented
recovery paths.

Every registration callback now carries a stable per-file identity, its exact
stream kind, and the retained `StreamPaths`. The reference's identity is the
live `cStreamFile*`; Rust uses pointer equality and hashing over a private
owned allocation and exposes no guessed numeric guest address. The nested
`GameStream` and containing `StreamFile` retain clones of that same identity,
so XSM, XST, XSS, XML, and all three unload callbacks remain associated even
for concurrent same-kind files with identical paths and stamps. This identity
capability is a high-confidence safe native ownership analogue; callback order,
path bytes, raw states, and failure behavior are unchanged.

XSS completion first reconstructs the file-local state in `cStreamFile::Update`
at `0x824e0d14..0x824e13dc`. The raw XSS count is stored at `+0x58` and resizes
the native vector to that logical length, while `mNumOctreeInfo +0x25c` counts
only the compact accepted prefix. The resize helper at
`0x824dfaf0..0x824dfb24` fills new slots from an uninitialized caller
temporary; safe Rust therefore retains the raw logical length and initialized
prefix separately and exposes only the opaque tail length, never fabricated
tail records.

Accepted `mBBox` values aggregate in compact source order over all four lanes.
Zero accepted records use `[-1,-1,-1,+0]` through `[+1,+1,+1,+0]`; both paths
then apply the exact `[200,200,200,+0]` vector at
`0x824e12b8..0x824e12f8`. Descriptor/allocation/Initialize use the raw logical
length, the returned local Octree identity is stored, and accepted records are
inserted by ascending compact index with `mBBox` before raw state 4. Concrete
Octree resource planning, initialization, routing, insertion, and leaf splitting
are now reconstructed in `octree`, together with the state-5 ascending compact
removal loop, pre-unlink saturated-count quirk, and single-level empty-node
collapse; see `octree.md`. Spatial queries and exceptional PPC vector payload
parity remain unrecovered. The safe construction/insertion/removal error
channel is an explicit high-confidence deviation from the native unchecked or
assert-and-continue paths.

`cAssetStreamSystem::UpdateStreamFileLoaded` at `0x824e1ad0` then observes
inner raw 4, publishes the accepted processed identities to the global map in
order, and inserts the retained file identity into its per-type vector using
unsigned first-NUL C-string ordering. Existing equal paths remain before the
new file. Only after both operations does the adapter expose loaded to the
outer controller, allowing its same poll to begin XML. Native first-ID-wins and
both-file publication on a collision are exact evidence; atomic global failure
handling is implemented by the concrete `world_stream_store` as a typed safe
deviation; see `world-stream-store.md`. Publication now consumes the adapter's
unique local-spatial value and returns a file-specific token. During state 5,
the final inner callback synchronously releases that token's local value before
the inner state machine can expose raw 6; only after raw 6 does the adapter
request global unpublication. A release failure therefore latches at raw 5,
while an unpublication failure latches with raw 6 visible and retains the token
and processed storage. The outer Rust file identity intentionally remains alive
until its owner drops the file. These result-bearing failure paths and the
extended identity lifetime are high-confidence safe ownership policies; the
valid-path order is exact. The concrete `world_stream_runtime` publisher now
connects the catalog, unique local Octree, and global store behind this
contract; see `world-stream-runtime.md`. Immediate
Presentation
`AddHighCollection` calls remain outside the transaction, with later-failure
cleanup ownership unknown.

## Reference BAM measurements

The supplied reference corpus reports environment version `515600` and 787
named BAM locations. The presentation and simulation tables agree internally
on their respective stamps:

| Stream | XST assets | XSM collections | XSS spatial cells | Stamp |
|---|---:|---:|---:|---:|
| Presentation | 16,292 | 285 | 284 | `0000000095462c2e` |
| Simulation | 2,135 | 284 | 284 | `000000009532ee28` |

The first spatial cell is the high-detail cell at grid position `-3500,-900`.
Its hull is `[-3600, 103.026, -1000, 1]` through
`[-3400, 103.026, -800, 1]` in both streams.

### Presentation cell

- Collection: `cPres_-3500_-900_high`
- XSF bytes: 2,176
- Asset ID: `240ca28e3e639899`
- Asset type: `ab329a6a`
- Serialized resource bytes: 3,520
- Resource class 0: RefPack-compressed from 1,507 to 3,292 bytes
- Resource class 2: 228 raw bytes
- RW4 arena ID: `a889aca4`
- RW4 version: `454.000`, 32-bit pointers, big endian
- Dictionary: 12 objects

### Simulation cell

- Collection: `cSim_-3500_-900_high`
- XSF bytes: 2,176
- Asset ID: `960409b555c06628`
- Asset type: `ab329a6a`
- Serialized resource bytes: 1,756
- Resource class 0: 1,756 raw bytes
- RW4 arena ID: `8646a1f7`
- RW4 version: `454.000`, 32-bit pointers, big endian
- Dictionary: 6 objects

The simulation dictionary contains type IDs `00eb0008`, `00eb000d`,
`00080001`, `00080006`, `00eb000a`, and `00eb000b`.
`ArenaSystem::RegisterArenaTypes` and `pegasus::RegisterArenaTypes` prove these
relevant mappings:

| Type ID | Native type |
|---:|---|
| `00eb000a` | `pegasus::tCModelData` |
| `00eb000b` | `pegasus::tTableOfContents` |
| `00eb000d` | `pegasus::tInstanceData` |
| `00080001` | `rw::collision::Volume` |
| `00080006` | `rw::collision::ClusteredMesh` |

The repeated `00eb0008` record has deliberately not been named because its
callbacks fold into generic no-op stubs and do not provide a unique type name.

## Collision mesh reconstruction

Direct executable evidence:

| Address | Function | Recovered rule |
|---:|---|---|
| `0x828b46e0` | `ArenaSystem::RegisterArenaTypes` | collision type IDs and fixup callbacks |
| `0x828b4698` | `ClusteredMesh::Fixup` | KD-tree and cluster-table offsets are mesh-relative |
| `0x82ae0860` | `Volume::RegisterArenaReadCallbacks` | `00080001` maps to `Volume::Fixup` |
| `0x82ac8e60` | `ClusteredMesh::GetVertex` | three vertex compression layouts |
| `0x82ac8f30` | `ClusteredMesh::GetUnitSize` | variable unit byte size |
| `0x82acf4d8` | `ClusteredMesh::GetUnitVolumes` | triangle/quad indices, edge data, group and surface IDs |
| `0x827bb850` | `ClusteredMeshBBoxQueryIterator::QueryRange` | KD child tests and encoded leaf ranges |
| `0x827bd340` | `AALineClipper::Init` | extended line origin/delta, reciprocal refinement, and padding |
| `0x827bd480` | `KDTreeLineQuery::KDTreeLineQuery` | strict padded-root clip and state initialization |
| `0x827bb9d8` | `ClusteredMeshLineQueryIterator::QueryRange` | reciprocal-directed near-first line traversal |
| `0x82ac8b70` | `KDTreeLineQuery::ClipEnd` | stable pending-interval compaction and end clipping |
| `0x827c1300` | gameplay collection `LineTest<TrajectoryLineTestFunctor>` | threshold gate, outer octree or ascending-slot provider, and complete traversal |
| `0x827c01a0` | gameplay per-mesh line test | KD exhaustion, unit expansion, strict nearest replacement, and hit counting |
| `0x827c1548` | gameplay collection `BoxTest<TrajectoryBoxTestFunctor>` | transformed local query, outer octree or ascending-slot provider, and fixed-cache stop |
| `0x827c0428` | gameplay per-mesh trajectory box test | KD exhaustion, unit expansion, final 13-axis world-space SAT, and cache admission |
| `0x82acfdd0` | `ClusteredMeshCluster::UnitGetOverlappingGPInstances` | transformed triangle construction and recomputed face normals |
| `0x827ace68` | `AABoxTriangleOverlapTestOpt` | final 13-axis trajectory world-AABB versus GP-triangle overlap |
| `0x825e9480` | `AABBoxTemplate::Transform` | center/half transform used before per-mesh culling |
| `0x827c8378` | `ClusteredMeshData::Reset` | `0xc0` record construction, inverse transform, root bounds, and owner |
| `0x827c84e0` | `GroundPool::AddVolume` | identity ground placement and slot-order append |
| `0x827c8c68` | `GroundPool::GroundPool` | fixed-capacity record allocation and null outer octree |
| `0x827ab338` | `World::LineCollisionTest` | batch acquisition, execution, and nearest-result copy-out |
| `0x827b2538` | `Island::LineCollisionTestWithVehicles` | tail-call to its owning `World` |
| `0x827c19c8` | `LocalClusteredMeshLineTest` | batch worker dispatch to the collection query at `0x827c1abc` |
| `0x82acecf0` | `ThinTriangleLineSegIntersect` | one-sided determinant and barycentric tests |
| `0x829c69b0` | `tInstanceData::Fixup` | instance links are arena IDs; strings are object-relative |
| `0x82bb57c8` | `rw::RwHash32String` | multiply by `0x01000193`, then XOR each byte |
| `0x82f76a08` | `kAssetType_Instance_DMO` initializer | hashes `"Instance_DMO"` from seed `0x811c9dc5`, yielding `0x06d051d1` |
| `0x82d5ec38` | `cProcessArenaAsset::ProcessInstanceList` | maps `Instance_DMO` records to registry asset type `0x0b` using `tInstance::m_uiGuid` |
| `0x828a01e0` | `Simulation::RegArena` | registers type-`0x0b` DMO models when userdata bit 3 is set |
| `0x82ce9c60` | `LivingWorld::ModelMan::AddDMOModel` | converts the registry ID to `tInstance::m_uiGuidLocal` and constructs the simulation model |
| `0x829c6518` | `tCModelData::Fixup` | collision-model meshes resolve through arena object IDs |
| `0x82ae1908` | `rw::physics::assembly::InitializeArenaReadCallbacks` | registers `Assembly` as `0x00040006` and `AssemblyDefinition` as `0x00040007` |
| `0x82ae17a0` | `rw::physics::Assembly::Serialize` | serializes the definition ID and object-relative part/joint/drive arrays |
| `0x82ae18c0` | `rw::physics::Assembly::Fixup` | resolves the definition ID and runtime component allocations |
| `0x82ae0bf8` | `rw::physics::Assembly::InitializeComponentsFromDefinition` | constructs one runtime part per serialized part definition |
| `0x82de8090` | `Dmo::Body::SetPosition` | applies each locator matrix directly through `Part::SetTransform` |
| `0x829c5e18` | `tTableOfContents::Fixup` | TOC entries, names, and object-ID links |
| `0x82bbfa50` | `Arena::IdToObject` | 10-bit section plus 22-bit dictionary index |
| `0x828b4488` | `KDTreeMappedArray::Serialize` | volume and KD allocations are object-relative |
| `0x82f766b0` | `TypeToStringTable` initializer | names Pegasus data and subreference type IDs |
| `0x829c5d30` | `tSplineData::Fixup` | header and segment links are object-relative |
| `0x829c5670` | `tDMOData::Fixup` | 128-byte DMO records, object-relative names, and cleared runtime instance pointers |
| `0x82d3ad38` | `LWDynamicObjectLocatorMan::CreateLocator` | copies streamed locator and simulation-model IDs into runtime locator state |
| `0x82d3be90` | `LWDynamicObjectLocatorMan::AddLocatorsForStreamedLocatorChunk` | derives persistence, priority-tracking, and static flags from the stream policy and asset flags |
| `0x82d3d6d0` | `LWDynamicObjectMan::AddStreamedLocatorData` | normal world DMOs allow persistence and priority tracking and are not forced static |
| `0x82d39f90` | `LWDynamicObjectModel::Create` | resolves an external simulation asset to its model `tInstance` |
| `0x82d29f58` | `LWDynamicObjectModel::FindCharacteristic` | maps the serialized default class selector to the runtime characteristics class and applies collection fallback |
| `0x82d34b68` | `LWDynamicObjectMan::GeneratePriorityIndexForLocator` | builds the five-field locator priority record from model, persistence, camera, and distance inputs |
| `0x82d2da38` | `LWDynamicObjectMan::IsLocatorHigherPriorityThanLowestPriorityObject` | compares `((other + safety) + priority) * multiplier` with a strict greater-than test |
| `0x825eab98` | `Dmo::Registration::RegDmoModel` | registers that model by `tInstance::m_uiGuidLocal` |
| `0x82d17340` | `Vehicle::InitSplines` | installs embedded model grind/grab splines on a live vehicle body |
| `0x82d2bf18` | `DynamicObject::InitSplines` | installs embedded model grind/grab splines on a live DMO body |
| `0x827b42c0` | `Island::InitWorld` | creates one offline `DMOPool` with capacity 50 and its own clustered-mesh collection |
| `0x827b6798` | `Island::AddDMOBody` | inserts only a spawned body's assembly into that global pool |
| `0x82e50cb0` | `cLivingWorldCollisionBody::AddToWorld` | dispatches assembly groups 14 through 16 to `AddDMOBody` |
| `0x8271c690` | `CensusMan::EvaluativeSpawnDynamicObjects` | queries up to 256 passive locators, then applies flags, priority, overlap, and the live cap |
| `0x82d3ad20` | `LWDynamicObjectLocatorMan::GetPassiveLocatorsWithinRange` | delegates the horizontal radius query to the passive-locator KD tree |
| `0x82d39670` | `KDNode::FindNear` | accepts Euclidean distance equal to the radius and limits traversal to the requested neighbor count |
| `0x827edf20` | `KDTree::FindNear` | returns candidate IDs in ascending 2D distance order |
| `0x82f5d7f0` | `kMAXDYNAMICOBJECTS` initializer | installs the default live-object limit of 49 |
| `0x829c5998` | `tLocationDescData::Fixup` | location, sublocation, and string links are object-relative |
| `0x829c6b20` | `tTriggerInstanceData::Fixup` | trigger collision models are arena IDs; names are object-relative |
| `0x824d73b0` | `cAssetStreamSystem::GetCollectionId` | exact 200-unit high-detail grid naming rule |

The native implementation now parses the exact debug layouts for `Volume`,
`ClusteredMesh`, `KDTree`, `KDTree::BranchNode`, and
`ClusteredMeshCluster`. Signed-32 vertices are absolute quantized coordinates.
Unsigned-16 clusters begin with a three-i32 quantized origin at byte 16 and
packed vertices at byte 28. Mode 1 zero-extends each packed `u16`, adds it to
the signed origin with `vaddsws`, converts that saturated signed-i32 result via
`vcsxwfp`, and only then multiplies by the mesh granularity. The readable
decoder now preserves this integer-before-float order with `i32::saturating_add`.
Only upper signed saturation is reachable because the packed addend is
nonnegative. Signed-32 values remain absolute quantized coordinates converted
and multiplied without an origin add.
Cluster unit and normal offsets are 16-byte slots relative to the start of the
vertex area.

Unit records implement the reference flag grammar: triangle, quad, or triangle
list; optional edge-angle bytes; optional one- or two-byte group and surface
IDs. Quad winding matches `GetUnitVolumes`.

The matching outer-provider extraction is
`world_probe_temp\outer_collision_line_followup_raw.txt`
(SHA-256
`A57DED62314111B33A8E570FE6CE328198BAFBF8D852CE4A49AA720704E456DF`),
generated by `dump_outer_collision_line_followup.py` (SHA-256
`E165689F63F44F612EA0D750A5D9537FDAB86CFF4373A4F02BA52DD6493F9717`).
PDB layouts fix `ClusteredMeshCollection` at `0x10` bytes with
`meshes/count/spacial` at `+0/+4/+8`, and `ClusteredMeshData` at `0xc0` bytes
with forward matrix `+0x10`, inverse `+0x50`, KD-root AABB `+0x90`, and owner
`+0xb0`. The normal world config creates a 128-slot GroundPool. Its constructor
leaves `spacial` null, and `AddVolume` appends identity-placed meshes, so ground
queries use ascending slot order rather than an outer octree.

Gameplay collection line queries at `0x827c1300` reject a segment unless at
least one absolute XYZ delta is strictly greater than `0x37800000` (`2^-16`).
For each slot, `AABBoxTemplate::Transform` reconstructs the query box through
center/half arithmetic even for identity, then the no-octree cull rejects iff
`mesh_min > query_max` or `query_min > mesh_max` is ordered true on any XYZ
lane. Bounds equality survives and W is discarded. Accepted meshes run
`ClusteredMeshLineQueryIterator::QueryRange` to exhaustion at `0x827c01a0`.
Every primitive hit increments the count, but the nearest owner changes only
on a strictly smaller clamped line parameter, preserving the first native
mesh/unit/triangle on ties.

`UnitGetOverlappingGPInstances` recomputes each gameplay face normal from the
forward-transformed vertices as normalized `(p1-p0) x (p2-p0)`; it does not
read the serialized cluster normal table for this field. The single-triangle
cross/normalize is at `0x82ad0160..0x82ad01b4`, its `GPTriangle +0x10` store at
`0x82ad02f0`, and gameplay copies that value at `0x827c035c/0x827c0384`.

This gameplay specialization never calls `KDTreeLineQuery::ClipEnd`: both the
outer provider and per-mesh iterator are exhausted. `ClipEnd` belongs to the
separate generic RenderWare `VolumeLineQuery` implementations
(`0x82ad9e84`, `0x82adf970`, `0x82adfd74`, `0x82adff08`, `0x82ae0178`, and
`0x82ae0510`). The public host all-hit API is separate again: it retains all
accepted hits and stable-sorts them, which is not a claim about native gameplay
iteration order or generic RenderWare nearest-query pruning. These statements
prove ordinary finite operation order and ordered comparisons; Xenon reciprocal
estimate bits, fused rounding, denormal behavior, NaN payloads, and exact
signed-zero propagation remain hardware-oracle boundaries.

The gameplay trajectory ground-box path is separate from the ordinary boolean
box batch. `TrajectoryBoxTestFunctor` is `0x1010` bytes: a `Triangle[64]` cache
at `+0x0000` and count at `+0x1000`; each stored `Triangle` is `0x40` bytes with
three vector positions and a surface ID at `+0x30`. The outer function at
`0x827c1548` identity-transforms the original world box into local/model space
for the stored KD/root-bounds cull and inner KD/unit queries. The final SAT at
`0x827c04e8` deliberately uses the original world box against the emitted,
forward-transformed GP triangle. Traversal is slot, KD-range, unit, then
triangle order. The 64th accepted overlap is stored and traversal continues;
only the first later accepted overlap detects the full cache and stops without
storing a 65th. The big-endian `lhz` at `0x827c0528` reads the surface from the
high half of the GP user tag at `+0x88`.

The host ground-box collector is failure-atomic and capped at 64, but it
exhausts visited KD ranges and eagerly validates cluster/unit data before final
SAT filtering. A malformed later serialized object may therefore be reported
before the native 65th-overlap stop point. The host SAT preserves the recovered
nine specialized edge-pair tests in edge/X/Y/Z order, followed by the box-axis
and triangle-plane stages, with strict contact inclusion. `f32::mul_add`
models the `vmaddfp`/`vnmsubfp` graph; nested FMA is only a host approximation
of `vmsum3fp`, so Xenon fused/VMX exceptional, NaN, denormal, and signed-zero
behavior remains outside the bit-parity claim.

### Full BAM simulation corpus

`--inventory-bam-sim` validates every simulation XSF, RW4 arena, dictionary,
collision object, KD allocation, cluster, vertex, and unit range. It also checks
all expanded vertices against their mesh bounds. A full-bounds native KD AABB
query is also required to return every declared entry in every mesh. The
supplied corpus passes with zero failures:

| Measurement | Count |
|---|---:|
| Collections | 284 |
| XSF bytes read | 99,672,448 |
| Asset blocks / RW4 arenas | 2,135 |
| RW4 objects | 31,934 |
| Collision volumes | 1,835 |
| KD-tree mapped arrays | 1,033 |
| Inline primitive volumes | 14,932 |
| Clustered meshes | 792 |
| Clusters | 21,503 |
| KD branch nodes | 1,238,312 |
| KD entries | 4,643,391 |
| Vertices | 3,119,407 |
| Units / triangles | 4,628,459 |
| Signed-32 clusters | 982 |
| Unsigned-16 clusters | 20,521 |
| Unsigned-16 vertices | 3,001,884 |

The inline mapped-array volumes are 401 spheres, 13,911 boxes, and 620
cylinders. Every mapped-array full-bounds KD query returns its complete volume
set. Native transform-aware primitive intersection probes all 14,932 and
produces 14,932 hits with no invalid dimensions or misses. The validated
collision ownership chain is
`tCModelData -> Volume -> (KDTreeMappedArray | ClusteredMesh)`.

An additional full-corpus mode-1 arithmetic pass measured signed quantized
origins from `[-2,800,000, -79,560, -2,200,559]` through
`[2,201,739, 303,682, 1,446,182]`. Across all 3,001,884 unsigned-16 vertices it
found zero lanes where integer-before-float and the former float-before-add
expression differed before scaling, zero lanes differing after granularity
scaling, and zero saturating additions. This establishes equality for the
supplied BAM Simulation corpus, while the corrected order remains necessary
for large/custom coordinates and is not an instruction-bit-exact Xenon float
conversion or multiply claim.

The dedicated stateful KD line traversal is now implemented separately from
the AABB iterator and wired into mapped-array line candidate selection and
clustered-mesh thin/fat segment broad phase. An independent 1,825-tree trace
and a separate production comparator agree on 886,115 emitted ranges and
4,804,738 leaf units, with an exact 70,139,064-byte public protocol signature
`57c03f40e465e5f7`. See [`kd-line-query.md`](kd-line-query.md) for native
layouts and arithmetic, the larger full-trace gate, reproducibility hashes,
and the bounded Xenon floating-point claim.

Pegasus parsing covers 15,722 `tInstance` records, 792 collision models, and
62,810 TOC entries. Local object links match their dictionary types. The other
51,843 links use RW4 section 2, the subreference dictionary, rather than an
external arena. All 15,722 TOC records of type `00eb0069` resolve exactly to a
160-byte `tInstance` boundary inside their owning `tInstanceData` allocation.

The runtime type-name table identifies `00eb0064` as `SPLINESUBREF` and
`00eb0069` as `INSTANCESUBREF`. Native spline parsing covers 819 containers,
36,058 headers, and 120,244 144-byte segments. All 36,058 spline subreferences
resolve to exact header boundaries, and every serialized head, tail, owning
spline, previous, and next offset resolves inside its allocation.

The PDB-named `tEmbeddedData` header is 36 bytes: four parallel count/offset
groups for hot points, grind splines, grab splines, and visual indicators.
Static BAM simulation contains no standalone embedded records. BAM presentation
contains 34 records whose 39 references are all hot points. Moving grind data
instead lives with actor models. The hashed `livingworld.big` table contains
450 RW4 arenas, 65 version-7 binary recipes, and 65 XML companions; all 685
paths reconstructed from the recipes resolve through the original BIG hash.
Thirty-two vehicle component arenas contain 194 embedded grind references. All
194 are section-2 subreferences with type zero whose base is a
`0x00eb0004` `tSplineData` object, and all land on exact header offsets. The
native header-range converter produces 564 grind primitives without failure.

`tDMOData` is a distinct streamed locator format: a 20-byte container followed
by 128-byte records containing transform, AABB, locator GUID, local GUID,
simulation-model ID, runtime-instance word, and name pointer. The complete BAM
simulation corpus has 815 containers and 3,591 records. Their names independently
encode render-model and simulation-model identities; 99 unique pairs validate
against the structured model field. The reference fixup clears every serialized
instance word before runtime registration.

The processed Simulation DMO pass preserves that container boundary.
`ProcessDMOData` (`0x82d5f4f0`) emits one kind-31 action under the full TOC GUID
and passes the whole fixed-up object; it does not flatten the records. The
owner-bound `SimulationDmoArenaPlan` reproduces that selection and lifetime
without exposing a fabricated host pointer. A versioned whole-BAM oracle over
all 284 collections and 2,135 arenas locks 815 affected arenas, 815 containers,
3,591 records, and signature `1c41b2057433ff8a`. It also proves one container
per affected arena, record counts 1 through 21, object spans 382 through 7,871
bytes, 815 unique container GUIDs, 3,591 unique record GUIDs and identity
triples, no aliased source spans, no zero identity lanes, no empty names, and
all 3,591 serialized runtime-instance words nonzero before native fixup.

The runtime adapter is additionally checked with an alternating live-policy
oracle over all 815 containers. Even global operations take the true-policy
Challenge-plus-registry branch and odd operations take the false-policy Living
World-only branch under userdata `7`; deregistration always takes Challenge
then registry without a policy read. Versioned host-oracle signatures are
`16f4eda4f9132804` for registration and `70d81a75f9752756` for deregistration,
with 815/815 source-identity matches and zero mismatches.

The corresponding model bank is the mounted `worldmisc.big` stream
`data\content\world\dmo\DMO\cPres_Global.xsf`, described by `DMO_Pres.xst`
and `DMO_Pres.xsm`. Its 125 asset arenas contain 93 `tInstance` records whose
discriminator is exactly `Instance_DMO` (`0x06d051d1`). `ProcessInstanceList`
registers each model under its primary GUID; `GetSimModelIDFromInstanecID`
then returns the local GUID at `tInstance + 0x68`. The locator model field
matches that local GUID for 87 of BAM's 99 used model IDs. The bank also has six
unused models, while 12 named BAM model IDs are absent, an explicit version skew
in the supplied corpus. Seventy-seven model instances contain embedded data:
all 470 grind references use section 2, subreference type zero, and a
`0x00eb0004` spline base. Every offset resolves and yields 700 native grind
primitives with no failures.

The same 93 model records serialize the default characteristics-class selector
`d7edbd362d7d2152` plus a model-specific collection. `FindCharacteristic`
resolves that selector to class `ccb7c06868eb1c87`; the model collections then
reference `livingworld_dynamicobject_priority` records. All 93 resolve without
a synthesized value: 29 use `PriorityIndex=1`, 26 use `200`, and 38 use `450`.
The 249 resident Rob's Park locators consequently distribute as 66 / 123 / 60
across those three indices.

The collision link in each DMO model is now resolved through its exact runtime
ownership chain. When `tCModelData::m_iNumMeshes == -1`, `m_pMeshes` is a
direct arena ID for serialized `rw::physics::Assembly` (`0x00040006`), not an
assembly definition. That object's definition ID targets
`rw::physics::AssemblyDefinition` (`0x00040007`); each definition part then
owns a volume/aggregate root. Definition part allocations are resource-relative
with 32-bit wrapping addition, so they may legally precede the dictionary
object. Following this chain across the bank resolves 93 templates, 93 parts,
and 14,451 native primitives with empty ownership and parse failure sets.

Runtime construction follows the same path: `Dmo::Body` creates an assembly
from its definition and calls `SetPosition(locator_matrix, part)` for every
part. Serialized collision matrices use only the affine 3x3 axes and
translation; unused W lanes, including element 15, can be zero. Native affine
composition therefore combines axes and adds translation explicitly instead
of using animation-style homogeneous multiplication. This preserves the outer
locator translation exactly.

The resident join is exercised against the normal four-cell Rob's Park
simulation focus. Those cells contain 249 DMO locators, all of whose model IDs
resolve in this build's bank. The 168 locators whose templates have embedded
grind data contribute 1,430 ordered assets / 2,361 primitives to a resident
catalog. All 249 catalog entries can instantiate 249 collision bodies/parts
and 35,626 aggregate volumes plus mesh triangles. They are not all live:
streaming creates passive locators, while only the census spawn path creates a
body and calls `AddDMOBody`.

The generated 40-byte `livingworld_census_ranges` PDB layout contains two
20-byte endpoints: no-spawn radius, spawn radius, cull radius, velocity shift,
and speed. The real VLT `dynamicobjects` collection stores `0/60/75/0/0` for
both endpoints. No resident Rob's Park locator lies within 60 horizontal units
of the spawn focus, so the active DMO pool there is exactly empty. For decoder
validation, a separately labeled single-resident probe requires grind spline
and collision body ownership to match; it returns 17 transformed spline
candidates and three of five geometry lines, with audio/physics surface
`30/1`.

The PDB names the global priority fields and the executable supplies their
exact lookup8 keys. The global `livingworld` constructor selects collection
`20f04d295272ee46`; its real values are camera-plane `2.5`, in-use `2`, model
priority `250`, safety `300`, and distance `450`. The native core now builds
the PDB-proven `tObjectPriorityIndex` fields and compares them in the original
floating-point expression order. A missing persistence record starts at safety
index 1; static locators use the reference `+2^31` priority sentinel and the
in-use factor. Normal streamed world locators start enabled, persistence-
enabled, priority-tracked, and nonstatic. Application-owned live DMO and
vehicle collision bodies now supply the exact overlap-owner lists directly to
the census priority gates and ordered cull plan. `LWBasicEntity` proves that
all living-world entities share one counter initialized to 1; an emitted ID is
the entity-type byte followed by that counter's low 24 bits, and a DMO's type
byte is 5. The first cull request immediately runs the entity callback and
removes it from entity/execution managers; duplicate requests are ignored, and
the next census update destroys all pending entities. Nonempty activation
remains guarded at ownership of that allocator and lifecycle across every
living-world entity in boot order. DMO object/core/collision stores are each 50
slots with exact fresh ascending allocation and LIFO reuse. Pending destruction
order can now be replayed from a reference object-store base: EASTL hashes raw
32-bit object pointers modulo 51 and visits buckets in ascending order.

The collision admission stages are now named down to their primitive test.
`BuildModelOverlapIDList` (`0x82d38ab0` for DMOs, `0x82d28a10` for vehicles)
loads the candidate model's first collision volume and transformed model AABB.
The manager query calls `LivingWorldView`, which delegates to the global DMO or
vehicle `DynamicPool`. `HorribleUglyVolumeVolumeTest` (`0x827cb218`) walks live
assemblies into 64-entry scratch arrays, while
`VolumeVolumeListOverlapTest` (`0x82e4bc00`) first rejects disjoint assembly
AABBs, recursively instances `SimpleMappedArray` leaves, and tests each pair
through `rw::collision::Overlap`. The candidate padding is `-0.05` against
DMOs and `0.0` against vehicles. Returned assembly owners become object IDs;
zero IDs are removed and duplicate IDs are suppressed in query order.

The primitive predicate (`0x82ad6188`) uses the recovered six-type
`GPInstance` dispatch table, then checks separating distance against both
primitive fatness values plus padding and requires a feature-intersection
prism. The DMO bank's actual leaf population is 24 spheres, 85 capsules, 448
boxes, 29 cylinders, and 13,865 mesh triangles, so every required source shape
is now measured rather than assumed.

The PDB and named construction methods also fix the 192-byte `GPInstance`
representation. Native `GpPrimitive` construction now preserves transformed
positions, active face/edge counts, box X/Y/Z directions, capsule Z direction,
cylinder Z/Y/X face storage, dimensions, radius/fatness, flags, triangle
points, cached-or-rebuilt normal, cyclic normalized edges and their three
lengths, and edge data.
Shape interval projections match the named methods at `0x82acddb8`,
`0x82acce30`, `0x82ad8218`, `0x82ad87f8`, and `0x82ace7d0` without prematurely
adding fatness.

The generic candidate builder at `0x82aca230` uses reverse face-normal order,
forward nested edge crosses above squared length `.001`, and
`1.1920929e-7` line/centre fallbacks. The latter is not an inferred language
constant: the initializer at `0x82f7a630` broadcasts exact bits `0x34000000`
into `VEC_EPSILON`. The generic selector at `0x82aca8a0`
keeps the first equal-separation axis and orients the largest interval gap from
the first primitive toward the second. This is implemented and tested as a
separation-stage API, not mislabeled as full overlap. Sphere/sphere is the only
first complete boolean pair because both maximum features are points and the
prism path unconditionally produces one point. The capsule maximum-feature
threshold is exactly `.05`; its result is an endpoint or directed centre edge,
and the recovered feature-edge clamp preserves distinct start/interior/end
region codes. Point/edge and point/face prism helpers also always succeed after
projection, so sphere overlap is complete against capsule, triangle, box, and
cylinder primitives in either argument order. Cylinder against sphere or
capsule uses the specialized candidate builder described below. The named
box/triangle and box/box SIMD separators are also reconstructed below, so all
primitive-type pairs now reach the shared overlap tail.

`Feature::BuildEdgePlanes` at `0x82ac8810` crosses each feature edge with the
extrusion direction, reversing that direction for clockwise winding, and uses
the same strict f32-epsilon normalization boundary. The complete
`GPTriangle::GetMaximumFeature` at `0x82ace110` is also native: strict alignment
greater than `.95` selects three stored or reversed cyclic edges, region `8`
or `0`, the triangle face normal, and three built clipping planes. Otherwise,
the reference's strict projected-vertex tie order and `.05` cached-edge tests
select one vertex or directed edge. The separately named triangle feature-type
classifier preserves its exact edge-bit region codes and comparison order.

`GPCylinder::GetMaximumFeature` at `0x82add438` is complete as a domain-level
builder. Raw VMX128 operands establish cylinder-local X/Y/Z as the stored
X/Y/Z basis before every active feature field is transformed back to world
space. Strict axial alignment above exact bits `0x3f7ff2e5` emits a fixed
eight-edge cap using radius, `sqrt(1/2)`, and a winding-dependent first vertex;
a negative cap flips winding and height. Axial magnitude at or below exact
`.02` emits the directed positive-to-negative axial edge and deliberately
normalizes the full query direction for radial support. The intervening rim
point explicitly clears axial Z before normalization. Cap clipping planes use
canonical local +Z rather than the near-axial query direction.

The sphere branch of the specialized cylinder candidate builder
(`0x82ad4ac0`) and strict chooser (`0x82adafb0`) is also native. It emits the
cylinder axis first and a nearest-rim radial candidate second. Its degenerate
cross test uses `f32::MIN_POSITIVE`, then preserves the executable's
sign-sensitive `axis.x > .5` choice between global J and I. The chooser keeps
the first equal separation and orients the winner from cylinder to the other
primitive; the reverse wrapper negates only that direction. This completes
boolean cylinder/sphere overlap. The chooser is exposed separately as a typed
candidate-list API so the remaining exact rim solvers feed the same verified
strict tie policy.

The shared `AddLineToEdgeCandidate` helper (`0x82acac68`) is also translated.
It emits the normalized line/edge cross for an infinite-edge parameter inside
the closed finite interval. Parameters strictly above or below
`middle +/- half_length` select that endpoint and a line double cross; exact
endpoints remain on the cross-axis path. Parallel line/edge directions use a
base-delta double cross, with both degeneracy decisions comparing squared
length strictly against `f32::MIN_POSITIVE`; coincident lines fall back to the
line direction. Raw operands establish that the clamped endpoint branch uses
the absolute endpoint directly rather than subtracting `line_base`, an unusual
but intentionally preserved reference behavior.

`RimToEdge` (`0x82ad3710`) is translated at the domain level as well. It orders
the finite edge endpoints by signed rim-normal travel, computes the strict
interior projection through `cross(cross(normal, edge), normal)`, and seeds two
nearest-rim pairs. Same-sign stationary metrics return the pair with strictly
smaller magnitude; a bracket instead runs at most ten refinements with the
reference's successively halved weighting term. The final strict magnitude
comparison owns ties on the upper pair. Squared radial lengths strictly below
`f32::MIN_POSITIVE` remain unnormalized, while a coincident final pair first
tries `cross(cross(rim-center, normal), edge)` and then the rim normal. The
one-edge capsule branch of cylinder `GetCandidates` now emits, in order, the
cylinder axis, positive-rim/edge result, negative-rim/edge result, and shared
line/edge result. Together with the common maximum-feature/prism tail this
completes cylinder/capsule overlap in both argument orders.

`RimToRim` (`0x82ad3b98`) and `AddRimToRimCandidates` (`0x82ad46d0`) are now
translated too. The pair solver retains its two strict geometric refusal
gates, radius-scaled `1e-7` tolerance, separate near-coplanar projection, and
asymmetric general construction. The latter brackets a signed stationary
metric and performs at most ten decaying-weight refinements; strict pair
ownership and both coincident-point fallback levels are preserved. The caller
only runs when `cross(axis2, axis1)` is strictly longer than
`f32::MIN_POSITIVE`, then conditionally appends cap pairs in `(+,+)`, `(-,+)`,
`(+,-)`, `(-,-)` order. Flipping exactly one cap reverses the normalized line
direction and flipping both restores it.

The complete cylinder/cylinder `GetCandidates` list is therefore native: the
first axis, second axis, two second-rim/first-edge candidates, zero to four
rim/rim candidates, two first-rim/second-edge candidates, then the shared
line/edge candidate. Raw call setup confirms the unusual detail that both of
the first two rim/edge calls use the positive second-cylinder axis as their
normal even though the second call uses its negative cap center. The strict
chooser and completed cylinder maximum-feature/prism tail now decide boolean
cylinder/cylinder overlap.

`NearestRimToEdge` (`0x82ad4a00`) chooses the cylinder cap for a finite edge
without a conventional closest-point solve. It evaluates the unnormalized
`cross(edge, cross(edge, edgeStart-cylinderCenter))` and selects the positive
axis only when its axial dot is strictly negative; equality selects the
negative cap. The selected cap centre uses the cylinder half length before the
existing exact `RimToEdge` solver runs.

The cylinder/triangle branch at `0x82ad587c` consequently has eight fixed
entries: cylinder axis, triangle face normal, three `NearestRimToEdge`
candidates, and three cylinder-axis/triangle-edge candidates. The edges follow
the GP triangle's stored cycle `point0->point2`, `point2->point1`, then
`point1->point0`. Each final call uses its edge start as base and sets both
finite interval values to `storedEdgeLength * 0.5`. The strict chooser and
completed feature-prism tail make cylinder/triangle overlap complete in both
argument orders.

The cylinder/box branch from `0x82ad4f68` through `0x82ad5878` has a fixed
16-entry list. It begins with the cylinder axis and all three stored box face
normals. For each cap independently, strict `0 > dot(cap-boxCenter, boxAxis)`
tests choose a signed box corner; zero or unordered comparisons retain the
positive extent. Three `AddRimToEdgeCandidate` calls cover the corner's full
incident edges in stored axis order. Raw call setup proves that both cap groups
pass the positive cylinder axis as their rim normal, including the negative-cap
centre. The final sign selection uses the cylinder axis rather than cap
position. It builds the three silhouette corners
`(-q0,+q1,+q2)`, `(+q0,-q1,+q2)`, and `(+q0,+q1,-q2)`, then emits their six
edges in axis order `(1,2)`, `(2,0)`, and `(0,1)`. These feed the exact shared
line/edge helper with `middle=-qi` and `halfLength=Ei`. The strict chooser and
completed maximum-feature/prism tail now make cylinder/box overlap complete in
both argument orders.

`FindBestSeparatingDirectionBoxBox` (`0x82ad2c58`) is an unrolled SIMD SAT
over 15 axes. Its temporary stack order exposes three first-box normals in
reverse storage order, three second-box normals in reverse order, then all nine
normalized edge crosses in forward nested order. The specialized routine has
no generic `.001` small-cross gate: it evaluates every normalized cross,
including the degenerate result of parallel edge pairs. Candidate gaps use
strict comparisons, retaining the original direction on an equal two-sided gap
and the earliest axis on an equal best separation.

`FindBestSeparatingDirectionTriBox` (`0x82ad31a0`) applies the same policy over
13 axes: triangle face normal first, the box's three normals in reverse order,
then the nine forward-nested triangle-edge/box-edge crosses. It likewise omits
the generic cross-length filter. `FindBestSeparatingDirectionBoxTri` at
`0x82adaf58` swaps the arguments and negates only the selected direction. With
the existing maximum-feature and feature-prism implementations, box/box and
triangle/box boolean overlap are complete, which closes all 25 ordered pairs
among the five GP primitive types.

Debug types fix the feature prism as two 16-entry point arrays followed by a
normal, signed point count, and unsigned normal-override flag. Point/point, edge/point, and
face/point construction now produces those typed contacts while preserving
projection, furthest-plane clipping, clamp, region-update, and argument-order
semantics. Edge/edge clipping is complete: its non-parallel path preserves the
strict f32-epsilon cross-direction gate and ordered two-edge clamp, while its
parallel path preserves selected-axis interval overlap, strict two-point
acceptance, double-cross point construction, endpoint ties, and region updates.
Face/edge clipping
is complete through its strict `.001` cross and `.05` parallel gates, opposite
clip/constrain cross orientations, two-point interval projection, collapsed
one-point fallback, parallel-constraint fallback, region mutation, and
conditional replacement normal. The generic sphere/capsule/triangle overlap
path is complete across its point, edge, and face feature combinations. The
generic face/face path emits contained vertices, paired edge crossings, and
opposing-plane projections in reference order. Its zero-contact maximin scan
keeps the first global plane tie and the last-vertex initialization tie, then
calls the complete `FindFaceFaceNonOverlapPrism` reconstruction. That helper
preserves previous-adjacent-edge preference, the strict `.05` alignment gate,
opposing interval order, `MIN_POSITIVE` endpoint regions, vertex/edge clamp,
argument-order restoration, and the conditional normalized contact-delta
override.

`GPBox::GetMaximumFeature` at `0x82acc360` is complete from its raw
`axis_corner_signs`, `axis_corner_index`, and `other_dir_indices` tables. Each
axis uses inclusive `abs(dot) >= .2` classification: two low-alignment axes
emit a face, one emits an edge parallel to that axis, and zero or three emit a
vertex. Strict projection improvements keep the first enumerated corner on a
tie. Face construction preserves its fixed starting corner, forward/reverse
winding, Y-axis sign correction, four lengths, clipping planes, and support
normal. This completes generic box/capsule overlap in both argument orders and
feeds the completed specialized box/box and triangle/box separators above.

All 93 serialized model AABBs are now retained with their collision templates.
The native `AABBoxTemplate::Transform` reconstruction (`0x825e9480`) computes
the local centre and half-extents, transforms the centre, projects extents
through the absolute affine 3x3 axes, and subtracts/adds the result. Runtime DMO
bodies expose these exact world broad-phase bounds; focused coverage includes a
rotated nonuniform box rather than translation alone.

For each overlapping live DMO, the census calls the concrete
`DynamicObject::FillPriorityIndex` virtual. The stack record matches the
PDB's 20-byte layout exactly. A candidate is rejected when
`candidate.safety + candidate.priority <= overlap.safety + overlap.priority +
0.001`. The scan makes no cull requests until every overlap passes. A survivor
requests DMO culls in query order, then all vehicle-overlap culls, then the
lowest-priority pool cull selected earlier, and finally spawns. Typed core
policy and focused tests preserve those strict comparisons and action order.

Location/trigger parsing adds 53 location descriptors and 10 trigger instances.
All `LOCATIONDESCSUBREF` and `TRIGGERINSTANCESUBREF` entries land on their exact
128-byte and 240-byte inline strides, and every trigger collision-model arena
link validates. Registration code proves the non-sequential trigger container
ID is `00eb0019`.

The Location producer itself uses raw container type `00eb0009`, not the trigger
type. `ProcessLocationData` emits one Simulation kind-21 asset per complete
container under its full TOC GUID. The owner-bound `LocationArenaPlan` preserves
that first-TOC selection, authored order, whole-object lifetime, and exact inline
record positions. A versioned whole-BAM oracle locks 284 collections, 2,135
arenas, 107 containers, 83 empty containers, 53 locations, zero sublocations,
and signature `024de4660f5fc535`; source spans range from 32 through 778 bytes.
The live Location-manager multimap and collection acknowledgement are not part
of this producer plan. See
[`location-arena-plan.md`](location-arena-plan.md).

The separate streamed runtime adapter now reacquires each of those containers
by full ID on both registration and teardown. It reproduces raw name hashing,
per-non-null-descriptor priority snapshots, stable insertion after equal hashes,
and first exact-pointer removal. Full-corpus runtime signatures
`2c83b407a844d816` / `8b32aa3e9943123b` cover 107 forward calls each way and
53 matched insert/remove pairs with no teardown priority reads or surviving
state. Its owner-retaining sorted vector is host storage, not a claim of native
fixed-tree allocation or complete Location-manager behavior.

The exact reference grid formula maps the `A_RobsPark` location to
`cSim_500_900_high`. The full non-XML focus policy uses a horizontal radius of
195, vertical half-extent 8,000, hull-center XZ distance ordering, and a default
simulation cap of four. It selects `cSim_500_900_high`,
`cSim_500_1100_high`, `cSim_700_900_high`, and `cSim_700_1100_high`, in that
order. Copying those collections into runtime-owned collision structures yields
68 arenas, 451 primitives, and 354,382 triangles. A downward query at the spawn
hits the decoded ground at y `-54.431` with normal `[0,1,0]` and surface ID 133.
Runtime queries use each mapped-array or mesh KD tree for broad phase before
applying the native narrow-phase tests.

The focus-selection and membership state machine is detailed in
[`stream-focus.md`](stream-focus.md).

## Confidence and open edges

The fixed layouts, byte order, offsets, counts, RefPack behavior, collision
geometry, primitive transforms, and object ranges are high-confidence because debug types,
decompiled named routines, and complete-corpus validation agree. XSF's three
unresolved header words and the semantics of several non-collision RW4 types
are retained losslessly but not guessed. Thin one-sided triangle intersection
and the native KD-backed AABB and line traversals are implemented; fatness is
forwarded into the clustered-mesh line query. The gameplay ground provider is
now attributed through its null-outer-octree slot loop, strict nearest update,
and fully exhausted per-mesh KD traversal. Generic RenderWare hit-driven
`ClipEnd` is a separate `VolumeLineQuery` behavior, not part of that gameplay
path. Remaining format work includes the embedded TOC object classes. Xenon
`vrefp`, FMA, denormal, NaN-payload, and signed-zero bit parity remain a
hardware-oracle boundary.
