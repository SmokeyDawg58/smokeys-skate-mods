# Rob's Park resident composition probe

## Scope

`--probe-robs-park-resident` extends the real BAM manifest probe before its
diagnostic stream-file teardown. It drives the 14 spatially selected Rob's
Park collections through one shared `CollectionResidentStreamer`, the
catalog-backed mounted driver factory, the paired decompression wake, and the
interim owned resident storage. Presentation and Texture retain complete
decoded RW4 arenas. Each Simulation resident owns one non-cloneable static
registration plan around the complete decoded arena and its unchanged
dictionary-order collision/grind projection; the plan separately owns its
native TOC-order, full-GUID grind registrations.

The probe first preserves the lower 14-collection resident-streamer gate, then
runs a fresh composition through the reconstructed parent manager. It still
does not perform complete native subsystem/message registration, publish a
queryable collision/render scene through this path, or enter a playable loop.

## Frozen selection and accounting

The location guard requires `A_RobsPark`, category `DIST_Waterfront`, and
position words `[44133726,c258ed35,4479204f]`. The mounted XSS tables are
queried with the recovered radii and caps. The probe guards their exact
single-precision encodings (`0x43430000` world, `0x428c0000` texture) and the
Presentation/Simulation/Texture caps (`16/4/16`) before selecting anything.
The resulting content order is:

- Presentation: 6 collections and 2,528 assets;
- Simulation: 4 collections and 68 assets;
- Texture: 4 collections and 685 assets.

Every collection ID, filename, `0x80` payload offset, asset count, stream
format, parent ID, and XSM payload size is checked. Per-kind descriptor and
asset-order FNV-1a signatures are checked using the frozen corpus algorithm.
The Presentation-then-Simulation-then-Texture grouping is a diagnostic probe
convention; only each kind's internal focus order is content-derived.

The successful real run asserts distinct byte domains:

- 3,281 asset occurrences, including duplicates;
- 73,508,371 stored compressed-arena body bytes;
- 74,129,920 full asset-stride bytes, equal to summed XSM payload sizes;
- 142,118,000 decoded destination resource bytes;
- 734,333 bytes for the largest individual stored body.

The 621,549-byte difference between stored bodies and strides consists of
record headers, offsets, alignment, and padding. The `0x80` XSF prefix is
outside both figures.

## Parent-expanded composition

The six selected Presentation children all point to the same BAM Presentation
parent, `cPres_Global` (`0xe87238efd7f02b45`). This is
`data\content\world\stream\BAM\cPres_Global.xsf`, not the separate DMO model
bank with the same filename. Its terminal parent is the exact invalid sentinel
`0xe171e1abfedb69ed`; its XSM record is source index zero, format 2, payload
offset `0x80`, payload size 1,298,304, and asset count 35.

An independent frozen corpus harness found 1,291,608 stored-body bytes,
3,485,480 decoded-resource bytes, and a 173,599-byte largest stored body. Its
descriptor and asset-order FNV-1a signatures are `8120a5fcfe642d8d` and
`015d8bdee33ff917`. The logical 1,298,432-byte XSF hashes to
`8da283fc34d3d0b025d32134b4683d9c10a237b90bf9e937a0caa10b6dcffb91`.
The frozen harness source and output SHA-256 values are respectively
`C4252B933A1D0ECE7CDBC7736AD60C06B5D2172CCEC9C675BA031725916A5E9D`
and `C9EE814224D5C50E4E14B32ED4CC99ED77FA98A2F03D2942077203D24FDE7589`.

The six ancestor-first Presentation plans plus eight terminal plans produce 20
logical load requests, 15 physical collections, and five parent cache hits.
Physical publication is parent, six Presentation children, four Simulation,
then four Texture collections. Exact parent-expanded totals are 3,316 arena
occurrences, 74,799,979 stored-body bytes, 75,428,224 accounted stride bytes,
and 145,603,480 decoded-resource bytes. The probe takes 383 serial load updates
and 368 decompression kicks; these remain host scheduling regressions.

The first parent callback observes resident reference one, installs manager use
one, and takes the two proven Simulation/Presentation registration holds. Five
cache-hit callbacks produce manager use six and resident reference eight.
Parent load callback references are exactly `1,4,5,6,7,8`; unload callback
references are `8,7,6,5,4,3`. All 20 unload requests drain on the first outer
unload update, but the manager consumes only one of six completed-parent FIFO
items per update. At manager use zero, recorded deregistration effects occur
Simulation then Presentation while the parent remains at reference two. Two
separate diagnostic subsystem acknowledgements then decrement `2 -> 1 -> 0`.
Their latency is host policy; no native subsystem work is claimed.

The parent builds first but is destroyed last, so expanded teardown checks the
flattened per-collection destruction order rather than incorrectly comparing
global build and garbage sequences. It destroys all 3,316 assets, removes the
14 selected collections followed by the parent from the exact-token registry,
and leaves zero residents, parent uses, FIFO items, acknowledgements, or
request leases.

## Ownership and completion checks

The composition constructor creates the factory and immediately obtains its
paired wake before moving both into the same streamer. Each session re-resolves
the complete canonical collection snapshot and owning stream kind. A
single-live raw-zero request lease is an explicit host policy; `-1` remains the
native ended sentinel and native request-ID allocation/reuse is unknown.

Every physical driver is wrapped only to copy its final read-only stream
snapshot and highest-stored scalar. All 14 complete in source order with zero
queued chunks, zero retained chunks, zero retained stride bytes,
`producer_finished = true`, and `producer_failed = false`. Each driver
independently checks its released chunk count against that collection's XSM
asset count, accounted bytes against its XSM payload size, and highest stored
block against the frozen mounted-producer corpus. Their snapshots also sum to
3,281 released chunks and 74,129,920 accounted bytes. The deterministic serial
cadence takes 375 load updates and 361 decompression kicks; those timing counts
are a host regression, not native thread scheduling evidence.

The diagnostic registry wrapper contains only an
`OwnedResidentCollectionView`; it cannot access assets or keep a collection
alive. Load callbacks observe reference count one after wrapper publication.
Same-order diagnostic unload callbacks run before the built-in decrement.
Teardown then produces 3,281 garbage-observer calls in the exact built asset
order, removes all 14 exact registry tokens, destroys all 3,281 owned assets,
and ends with zero residents. These registry and garbage effects validate the
reconstructed lifetime protocol; they are not native asset-manager
registration or garbage collection.

## Simulation projection

The 68 Simulation arenas retain 954 RW4 dictionary objects and project 71
aggregate roots, 45 primitive sets with 451 primitives, 26 meshes with 354,382
triangles, 26 collision models, 477 instances (26 collision-linked), and 42
grind assets with 10,314 primitives. These values match the independent frozen
projection harness.

## Static Simulation registration plan

The first used-dictionary TOC in each of the 68 arenas expands to exactly 42
direct grind actions and 26 accepted Environment actions in authored order.
The resident probe validates every operation's TOC/dictionary/native-grind or
flattened-instance locator, then hashes each collection boundary, arena
identity, operation kind, source object ID/inline index, direct grind
GUID/name/counts, Environment identity/model references, and collision-model
root sequence with the frozen big-endian host FNV-1a policy. Both the direct
and parent-expanded runs must reproduce
`39413e2d647f8631`; counts alone cannot satisfy the gate.

The plan keeps the existing dictionary-order, arena-ID-labelled grind
projection unchanged and owns the direct full-GUID grind assets separately.
This prevents an ordinary `SimulationCollisionAsset` returned by `into_asset`
from silently changing meaning, while operation indexes address only the
native-registration vector. The resident asset now prepares the exact
supported `{kind 6, kind 10}` subsequence and registers it forward through the
bounded static adapter. Kind 6 reaches a diagnostic call recorder because
concrete static `GrindData` ownership is still open. Kind 10 reaches the
empty, zero-status Environment unlock router, so the absent-context branch
immediately publishes all 26 roots. An explicit registration occurrence
retains the Environment sources; asset `Drop` consumes it through the same
forward deregistration sequence. Both direct and parent-expanded runs require
68 registered then 68 deregistered supported operations, identical 42-entry
grind add/remove ID sequences, 26 live identity-placed roots, and zero roots
after teardown.

That build/`Drop` coupling is diagnostic timing, not the recovered SimReg/DReg
message lifecycle. Full processor dispatch, the missing unlock context
producer, static `GrindData` storage, native batch synchronization,
assembly-backed publication, and collision-query wiring remain outside this
bounded milestone.

The resident Simulation owner now additionally retains a unified supported
normal plan. It merges the static 68-operation subsequence with 201 ID-dispatched
records in authored TOC order: 145 WorldPainter, four RAIN, four AI path, three
waypoint, and 45 SpatialMap records; the selected cells contain zero NavMesh
records. The app independently re-resolves every source and requires 269 total
operations plus descriptor signature `415b55e790aabfb0` in both direct and
parent-expanded runs. This is a processed-record planning gate only; the six
new manager domains (five exercised here) are not invoked yet.

## Confidence and remaining work

The reference-data identities, per-kind order, static-plan ordered signature,
counts, byte totals, decoded contents, final stream ownership counters, parent
plans/reference transitions, and observed resident callback/teardown order are
**exact** for this corpus.
The serial cadence, owned Rust asset substitution, request lease, weak
registry, callback enum, deferred acknowledgements, and telemetry are **high**
or explicit host policy. The supported kind-6/kind-10 adapter order and calls
are **exact**; its token/failure ownership is **high** safe-host policy. Native
allocator/asset handles, the other processed Simulation kinds, actual
Presentation/Texture registration, collection message timing, full world
publication, rendering, and playable freeroam remain unimplemented.
