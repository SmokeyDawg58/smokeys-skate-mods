# Stream focus and collection lifecycle

Last updated: 2026-09-03

## Reconstructed behavior

The native implementation treats the reference executable as a specification
for a small domain state machine. It does not retain guest registers,
instructions, or the Xbox octree memory representation.

| Reference routine or value | Address | Recovered behavior |
|---|---:|---|
| `cAssetStreamSystem::AddStreamFocus` | `0x824dacb0` | Initializes position, dirty/stalled state, radius, status, and invalid current ID |
| `cAssetStreamSystem::MoveStreamFocus` | `0x824d78b8` | Exact position comparison plus the reference lock rule |
| `cStreamFile::FindCollections` | `0x824df068` | Focus-box overlap, XZ distance, nearest-first ordering |
| `FocusLODDistanceCompare` | `0x824d6778` | Strict ascending `mDist` comparison |
| `tStreamFocus::ProcessEntries` | `0x824deb60` | Sort by 64-bit asset ID and merge old/new membership |
| `tStreamFocus::FindCollections` | `0x824e04e0` | Records nearest current ID, reconciles membership, updates status |
| `cStreamFile::Update` | `0x824e0ba8` | Computes each collection's signed stream-order key |
| `cAssetStreamSystem::AddLoadRequest` | `0x824dda08` | Cancels a matching pending unload, otherwise queues a load |
| `cAssetStreamSystem::UpdateLoadRequest` | `0x824d7310` | Rejects the invalid ID, then updates the first pending load with the same focus handle |
| `cAssetStreamSystem::AddUnloadRequest` | `0x824ddbb8` | Cancels a matching pending load, otherwise queues an unload |
| `cAssetStreamSystem::LoadBestCollection` | `0x824dbcc8` | Forward-scans for the first ordered-strict minimum below `1.0e9f` |
| `cAssetStreamSystem::LoadNextCollection` | `0x824dbe78` | Selects subsequent loads in stable ascending stream order |
| `cAssetStreamSystem::UnloadNextCollection` | `0x824dbfc8` | Takes pending unloads in FIFO order |

Recovered constants:

| Meaning | Value |
|---|---:|
| Presentation/simulation horizontal radius | `195.0` |
| Texture horizontal radius | `70.0` |
| Focus-box vertical half-extent | `8000.0` |
| Default simulation collection cap | `4` |
| Default presentation collection cap | `16` |

The non-XML path tests a focus-centered AABB against the hull of every XSS
entry admitted to the stream octree. Candidate position is the hull center;
distance is computed only in XZ. Sorting occurs before the stream-type cap, and
the nearest result becomes the current collection.

The reference octree only accelerates AABB overlap. `CollectionSpace` currently
performs the same overlap over all serialized hulls, which preserves results
while allowing a purpose-built native spatial index later.

Each admitted collection receives the exact signed key recovered from
`cStreamFile::Update`:

```text
x = floor(hull_center.x + 0.5)
z = floor(hull_center.z + 0.5)
stream_order = 10000 * x + z / 10
```

The final division is signed integer division and therefore truncates toward
zero. This order is independent of focus distance.

## Membership merge

The nearest-distance order is used to establish the current ID. The selected
set is then sorted by its full 64-bit asset ID. `StreamFocusState::reconcile`
merges that set with the previous ID-sorted membership and produces:

- `Unload` for an ID present only in the old set;
- `Load` for an ID present only in the new set;
- `Update` for an ID present in both, carrying its new distance.

Requests remain in the same asset-ID merge order as the reference. The native
collision scene separately retains the distance-ordered collections, with each
collection owning its decoded arenas. This makes collection removal explicit
and avoids a flattened geometry pool with ambiguous lifetime.

The request queue applies the next layer of reference behavior. A load cancels
a pending unload of the same ID, and an unload cancels a pending load. Loads are
inserted stably by signed stream order, but the first load may be extracted by
minimum focus distance; unloads retain FIFO order. An update changes the focus
distance on a pending load without changing its stream-order position.

Crucially, `UpdateLoadRequest` does not search by the supplied collection ID.
After comparing that ID only against exact invalid sentinel
`0xe171e1abfedb69ed`, it finds the first request whose native `mHandle` equals
the originating `tStreamFocus*` and overwrites only the distance at `+0x0c`.
Two pending loads from one focus therefore update the first vector match even
when the supplied ID names the later load; the same ID from two focus handles
remains distinguishable.

`FocusHandle` models that identity with a private shared allocation. Clones
compare and hash by allocation identity and debug output exposes no host
address. Native pointer-keyed behavior is exact; the `Arc` representation is a
high-confidence safe PC ownership policy. Opposite-direction cancellation
continues to compare the full 64-bit ID, same-direction requests are not
deduplicated, and equal signed stream-order keys insert stably after equals.
Both `AddLoadRequest` and `AddUnloadRequest` also return on the exact invalid ID
before searching for an opposite request, so an invalid request cannot enqueue
or cancel anything.

The first-load comparison is exact at its floating-point edges.
`LoadBestCollection` loads raw float `0x4e6e6b28` (`1,000,000,000.0`) as its
initial minimum, scans forward, and replaces the winner only when `fcmpu`
followed by `bge` observes an ordered strict-lower distance. Equal values keep
the first request, `+0.0` and `-0.0` tie, and NaNs never replace. A distance
equal to or greater than the sentinel is not eligible. Native code asserts if
no candidate replaced the sentinel; the safe Rust queue returns `None` and
leaves every request in place rather than continuing through the reference's
debug-break path.

## Validation

At `A_RobsPark` (`[588.8617, -54.231647, 996.5048]`) the selector returns:

1. `cSim_500_900_high`
2. `cSim_500_1100_high`
3. `cSim_700_900_high`
4. `cSim_700_1100_high`

The independently recovered 200-unit grid calculation also returns
`cSim_500_900_high` as current. Initial reconciliation emits four loads and no
updates or unloads. Unit tests cover subsequent mixed unload/load/update
merges, the unusual reference lock behavior, first ties, both signed-zero
orders, unordered NaNs, and the exact sentinel boundary.

Additional adversarial tests cover cloned and distinct focus identity,
address-free debug output, first handle-match update with two pending loads,
equal IDs from distinct handles, invalid-ID early return, high-word-sensitive
64-bit cancellation, same-direction duplicates, and stable equal-order
insertion.

A real-data round trip now moves the focus 200 units along +X. The resident set
becomes `cSim_700_900_high`, `cSim_700_1100_high`, `cSim_900_900_high`, and
`cSim_900_1100_high` (63 arenas and 326,273 triangles). Moving back restores the
original four IDs and reproduces the exact original nearest collision hit,
including its collection/arena/mesh source path.

## Remaining boundary

The exact outer three-kind scheduler now owns request priority, one-active
operation, completion FIFO, and activation/deactivation sequencing; see
`collection-stream-scheduler.md`. The remaining boundary is a result-bearing,
offset-aware asynchronous XSF backend plus its resource-registration and
collision-island callbacks. The current code does not yet claim that a live
player drives streaming.


## September25 selected live XML continuation

Fixed the frame-1 TerrainResident failure caused by rejecting every nonempty
Stream XML file. The selected terrain owner now parses the retained simulation
file once and uses its authored neighborhoods when live kEnableStreamXml is true.
The existing CollectionSpace distance/position calculation is shared; existing
resident loading and registration owners remain unchanged.

IDA evidence rechecked this turn:
- Generated StreamTiles parser82964ad0, StreamTile parser82961a50: StreamTile
  children with Center Vector2 and repeated Tile Vector2.
- ParseXmlStreamTile824e0830: floor(component+0.5), format
  c%s_%.f_%.f_high, existing RwHash64String; reject unavailable center, retain
  center first, inspect first16 neighbors, skip unavailable IDs and duplicates.
  Native16-slot capacity assertion occurs before duplicate detection.
- cStreamFile.Update824e0ba8: later valid definitions replace earlier mappings.
- cStreamFile.FindCollections824df068: nearest XZ center from a hull query
  with half-extents5,8000,5; raw constants820126ec/820126f0 confirmed. Look up
  that center's authored neighborhood, stable distance sort. No match/empty
  result falls back to the existing195-radius query.
- cAssetStreamSystem.FindCollections824dfea8: offline simulation cap applies
  after sorting unless focus-specific mUseStreamXml is enabled. This is separate
  from cStreamFile XML and remains false in this selected offline path.

Validation: 3 focused XML tests PASS, including actual debug-build archive data:
BAM_Sim.xml172291 bytes,273 valid neighborhoods. Spawn588.8617,-54.231647,
996.5048 and moved701,-54.231647,996.5048 each resolve a9-entry authored
neighborhood and select4 nearest. Original XML loaded through existing MountedBig;
XSS from worldbam.big. Two initial corpus attempts failed only because the test
looked for XSS at the wrong virtual/loose path; corrected to original archive.
Existing spatial-selection regression also PASS. This is source/corpus evidence,
not a reference runtime trace or proof of all streaming parity.

Scope: one owning simulation space; no cross-world XML collection resolution,
focus-specific explicit XML overrides, native unload/GC or overlapping arena
registration ownership added. Malformed/unsupported XML remains an explicit
error. KnownAir landing prediction and intermittent pushing remain unresolved.

Release-mode typecheck `cargo check --release -p skate2-pc -j 1` PASS;
explicit Cargo exit-code confirmation0. Existing warnings remain. Changed Rust
format checks PASS. No rebuilt executable or live controller run this turn;
build-release.bat rebuilds it.
