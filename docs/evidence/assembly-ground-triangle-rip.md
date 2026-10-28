# Assembly/ground selected-unit rip and triangle cache

This checkpoint reconstructs the source-preserving selected-range owner above
the ordinary Assembly/ground Contact producer:

1. lazy selected-range clustered-mesh unit traversal; and
2. old-overload GP construction, final world AABB testing, and the private
   twenty-`GPTriangle` cache chronology embedded in
   `AssemblyTriangleIntersectionTester`.

It constructs the complete represented semantic triangle records but
deliberately does not yet turn them into or publish Contacts.

## Authority

- native specialization:
  `RipTriangles<BBoxTestRipFunctor<AssemblyTriangleIntersectionTester>>` at
  `0x827cd7c8..0x827cd960` (`0x198` bytes);
- `AssemblyTriangleIntersectionTester::RunIntersectionTests` at
  `0x827cd1e0..0x827cd7c8`;
- executable/PDB/MAP SHA-256:
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`,
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`,
  and `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`;
- complete Rip proof packet SHA-256:
  `0BE0084A2C6160DFD562BBB8A51627EB6CC994694501A25ED430E7E53A05F6B0`;
- raw Rip text and binary span SHA-256:
  `B4FC2DF4223001B753A1B702BDB9478DF281C4CD9108B1C925A40273018C3A8E`
  and `2F17316C6CF45AB09F376CBCED57B1AA8BBDA705D199711FC47229A785E16859`;
- independently audited `collision.rs` SHA-256:
  `53D6ECB708A4D0158956F5B70EEDAC95DF36469783B7EB0CB92BF8624A8B8CB7`;
- audited `simulation_assembly_ground_triangle_cache.rs` SHA-256:
  `45F16C7F5A73AF9D475E0D7F55226082F957D293E609C46AED9B1AAB875646E7`;
- directly invoked `simulation_assembly_ground_triangle_topology.rs` SHA-256:
  `BE05D5A364F596B0C00FF32A0E78F3075ADE3E190A3180883B98DEE230D358F6`;
- audited `simulation_assembly_ground_triangle_rip.rs` SHA-256:
  `7691878261ED630475AE2B3AA2ADF77A3291A420380B4142FA965EDF24E1787A`.

The corrected function end is `0x827cd960`; `0x827cd858` is only the
`AABoxTriangleOverlapTestOpt` call inside the function.

## Selected-range unit visitor

`ClusteredMesh::visit_leaf_range_units` keeps the low-level unit context that
the older flattened `MeshTriangle` API discards. For a nonempty KD leaf range
it:

1. applies the existing 16/20-bit encoded cluster/unit split;
2. validates and parses only the selected cluster-table entry and cluster;
3. begins directly at the encoded byte offset without decoding or inventing
   boundaries for preceding units;
4. parses exactly `unit_count` sequential units and advances by each returned
   encoded size; and
5. propagates `ControlFlow::Break` before inspecting a successor.

A zero count returns before validating even a nonsensical encoded start,
matching the native rip-loop gate. Errors from a later selected unit preserve
the already-observed visitor prefix. Existing eager/flattened query APIs remain
unchanged for their current trajectory and grind consumers.

Tests use variable-size triangle, quad, triangle-list, group-ID, surface-ID,
and edge-byte records. They freeze mid-stream starts, exact-count tail
avoidance, early-break laziness, invalid selected clusters/offsets, and later
malformed-unit prefix chronology. A deliberately invalid unselected cluster
proves the visitor does not decode unrelated data.

## Twenty-triangle local owner

The native tester stores `numTrianglesInCache` at `+0x3c` and twenty
`0xc0`-byte GP instances beginning at `+0x40`. One accepted triangle follows
this exact visible order:

1. execute the optional debug-publication path;
2. if and only if the pending count equals twenty, run intersections on the
   old twenty and then unconditionally reset the count;
3. increment the pending count;
4. copy the current triangle into the reserved slot; and
5. wrapping-increment the total accepted-triangle counter.

The private host owner accepts a caller-supplied debug/no-op hook for step one
and an infallible value-level intersection seam for step two. The latter
borrows the actual initialized slot prefix; it performs no manufactured
twenty-record copy and cannot observe stale slots above the count.
`GpPrimitive` is now a `Copy` value, matching the pointer-free native
`GPInstance::operator=` record and allowing it to inhabit this owner directly.

`RunIntersectionTests` is native `void`. If a full nonflushable Contact buffer
causes it to stop early, Rip still resets/discards the old twenty and appends
the triggering triangle as slot zero. A focused test freezes that loss path.
Other tests freeze first-twenty accumulation, debug-before-flush ordering, the
twenty-first boundary, stale-slot exclusion, total-counter wrapping, and empty,
partial, and exact-twenty final drains.

The final job-tail drain consumes the host owner. This prevents a replay that
the native caller cannot perform: it calls once and then ends the stack
tester's lifetime.

## Integrated selected-range owner

The private `AssemblyGroundTriangleRipper` now joins these proven pieces
without adding a flattened or eager bridge. Native `BoxTest` copies `mBBox` and
`mSpew` into each functor at `0x827cd974..0x827cd9a8`, but rereads the tester's
`sendBackTriangles` field for each accepted GP at `0x827cd874..0x827cd880`.
The Rust owner snapshots all three for its lifetime as explicit host policy;
that is sufficient for the recovered production route, whose configuration is
stable and whose `AssemblyGroundCollisionJob` passes literal `spew=false` at
`0x827cdea4`. It also owns one typed cache. Each range call uses
`visit_leaf_range_units`, invokes the old free-overload builder with
literal-zero `volumeRef`, and visits each initialized sibling in ordinal order.

The two native boxes remain distinct. `BoxTest` inverse-transforms the input
box for mesh/KD selection at `0x827cd9c8..0x827cd9dc` and passes that local box
as Rip's formal bbox argument, but Rip never reads the argument. The final SAT
instead uses the original/world `0x20`-byte bbox copied into the functor at
`+0x10` and each GP's forward-transformed world points
(`0x827cd848..0x827cd858`). The owner therefore always computes
`triangle_aabb_overlap` against its original/world box. The `overlap || spew`
source expression preserves the native short circuit: true overlap does not
consult the force flag; failed overlap may still enter the complete accepted
path.

Debug enablement is independent of acceptance. When disabled, the callback is
a no-op but the local cache is still updated. When enabled, the existing cache
owner guarantees debug publication before an exact-old-count-20 drain, reset,
trigger append, and wrapping accepted-count increment. The same owner survives
multiple selected ranges and different mesh calls. Its consuming `finish`
models the caller's one job-tail `RunIntersectionTests`, not a drain performed
by native Rip itself.

The builder's scalar encoded-size return is intentionally not traversal
authority. Native calls `DoSizeCalculation` after every emitted-primitive loop;
the selected visitor advances by its parsed size only after the callback
continues. A callback break is used solely to surface a typed unit-construction
refusal. Contact-capacity exhaustion cannot break traversal because native
`RunIntersectionTests` is void.

Ten integrated tests freeze zero-count laziness and an empty consuming drain,
far geometry reaching the final predicate with both spew outcomes, transform
before the real world predicate, distinct real-SAT outcomes for a quad's two
sibling point sets, per-sibling rejection followed by a later unit, forwarding
both 16- and 20-bit encoded nonzero cluster/midstream starts, Assembly's
literal-zero `volumeRef`, debug-disable/cache independence, an exact
twenty-first flush spanning ranges and meshes, and retained prefixes for later
format and old-builder errors. The cross-range trace places the triggering
debug callback before the old-twenty drain and leaves that trigger alone for
the terminal drain.

## Deliberately separate boundaries and policy

The optional global debug-triangle destination's atomic exact-full clamp is
proven in the packet but is not owned here; the debug hook keeps that optional
branch outside physics-critical local state. The private 50-Contact temp
buffer and global append already exist in their separate typed owner.

Malformed selected data has no native recovery contract. Rust format and unit
errors are typed host policy: they retain all already-executed predicate,
debug, drain, and cache-prefix effects and stop before effects for the failing
unit. The owner does not claim rollback or native post-DebugBreak behavior.
Its source tuple is mesh-local cluster/unit/ordinal identity, not a globally
unique cross-mesh key.

Native Rip always returns literal zero and BoxTest ignores it. The Rust `Result`
exists only for safe-host parser/builder refusal and must not be interpreted as
a recovered processed count or native abort token. Exact malformed output-count
behavior, optional debug-buffer atomic races, more-than-256 KD descriptors,
cross-job scheduling, W lanes, and Xenon exceptional floating point remain
unclaimed.

The next stage is the already isolated
`AssemblyTriangleIntersectionTester::RunIntersectionTests` body. The complete
private `Sk8ComputeContactPoints` plus triangle-fixup/final-pack value path is
now available, as is the pure sixteen-row final-value compositor. The remaining
owner must route these batches through its exact gates and the separately
proven effective-padding graph, bind live occurrence-qualified body roles, and
preserve reserve/write/null-abort chronology into the shared temporary Contact
buffer. The opaque reserve-first write capability now preserves that local
mutation ordering, but its live issuer, point loop, and job-tail flush remain
open. See `docs/evidence/assembly-ground-contact-write-port.md`.
