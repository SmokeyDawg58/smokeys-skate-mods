# Globally interleaved normal Simulation arena adapter

## Scope and provenance

`SimulationNormalArenaRegistrationAdapter` is the first runtime seam that
executes every currently reconstructed normal Simulation kind in one authored
operation vector. It covers kinds 6, 10, 11, 16, 22, 23, 25, 27, 28, and 32.
It does not invoke the older filtered adapters as grouped passes.

Reference inputs are:

- XEX SHA-256
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`;
- PDB SHA-256
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`;
- MAP SHA-256
  `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`;
- external dispatch packet SHA-256
  `A9D58D47623F59C478AE220859EE3F7A0333DBC1887C580AC41D222178B60207`;
- independent formats-only raw oracle source SHA-256
  `47D026FAEDF43E2C43712989B0DFC4AF9E7608934D00A37F2ECB08F1CEF30901`;
- external full-preparation sweep source SHA-256
  `49B14F2E2C0FEA5E421C73A094971A98FC920E04FEF303773F3AF4C41C96E406`.

The executable, symbols, decompiler database, proof packets, game data, and
external harnesses remain outside the repository under
`world_probe_temp`.

## Exact recovered dispatch

`Simulation::RegArena` at `0x828a01e0` and `DRegArena` at `0x828a05f0`
walk the same processed asset-pointer vector forward. They do not group by
kind and deregistration is not reversed. Each arena call receives one `u32`
userdata snapshot. Only kind 11 tests mask `0x8`, independently in each
direction.

| Kind | Registration | Deregistration |
|---:|---|---|
| 6 | `GrindData::AddSplineData` `0x82cfd228` | `RemoveSplineData` `0x82cfb980` |
| 10 | `UnlockableLocationMan::AddEnviroInstance` public/private `0x828b02d8`/`0x828b0068` | `RemEnviroInstance` `0x828b0188` |
| 11 | `ModelMan::AddDMOModel` `0x82ce9c60`, then DMO register | `ModelMan::RemoveDMOModel` `0x82ce8d68`, then DMO deregister |
| 16 | trigger add `0x82e24810` | trigger remove `0x82e22cb8` |
| 22 | streamed-tile register `0x82ceb770` | unregister `0x82ceb268` |
| 23 | NavMesh add `0x82ce6a80` | remove `0x82ce6ac0` |
| 25 | RAIN add `0x827202d8` | remove `0x82727510` |
| 27 | AI-path add `0x824d4890` | remove `0x824d0f78` |
| 28 | waypoint register `0x82719e60`, literal `true` | deregister `0x82722088`, literal `false` |
| 32 | SpatialMap add `0x82dfe480` | remove `0x82dfe3f8` |

All applicable IDs remain full `u64`. Kind 10 deliberately routes by the
record's local GUID through the concrete Environment router, rather than its
processed-asset ID. Kind 11 passes the primary GUID to the model call and the
same primary GUID plus exact retained 160-byte record to the DMO call. Kind 16
passes the exact retained 240-byte record. All callees return `void`; the outer
functions expose no rollback or status channel.

Preparation validates every operation before effects: static operation and
grind indexes, complete Environment sources, pointer record kinds and lengths,
and both serialized identity lanes. One strong occurrence owner is installed
before the first call. Registration and deregistration each walk the resulting
vector once, forward. A fresh deregistration userdata argument intentionally
allows the native clear/set and set/clear kind-11 asymmetries.

## Host safety boundary

The native code has no recoverable manager-error protocol. The Rust boundary
therefore catches a collaborator unwind or concrete Environment capacity error,
records the exact reached operation/stage, retains every potentially referenced
source, and permanently faults that adapter domain. It never invents rollback.
A reached registration fault returns no teardown authority; a reached
deregistration fault consumes it. Pre-effect allocation, exhausted-ID,
foreign-domain, unknown-token, and already-faulted errors have no manager
effects and preserve authority where one exists.

If kind 11 was added with bit 3 set but removed with it clear, the complete
source owner remains until adapter destruction because the native DMO manager
may still hold the record pointer. Dispatcher and Environment manager fields
are destroyed before live/orphan source ledgers. Tokens and adapter `Drop`
perform no hidden manager cleanup. These allocation, authority, unwind, and
sticky-fault rules are high-confidence safe-host policy, not native exception
behavior.

Ten focused tests cover exact mixed-kind forward order in both directions,
independent userdata cases, repeated and empty occurrences, foreign authority,
ID exhaustion, every dispatch-stage registration and deregistration prefix,
pre-existing token behavior after a later fault, Environment store-before-error
state, source retention/release, and destruction order.

## Independent whole-BAM gates

The formats-only V1 oracle does not import `skate2-core`. It traverses the raw
XSM/XSF/RW4/TOC data and reports:

| Kind | Operations |
|---:|---:|
| 6 | 819 |
| 10 | 790 |
| 11 | 0 |
| 16 | 10 |
| 22 | 6,122 |
| 23 | 0 |
| 25 | 75 |
| 27 | 101 |
| 28 | 95 |
| 32 | 1,033 |
| **Total** | **9,045** |

It covers 284 collections, 2,135 arenas, and 62,810 first-TOC entries. Its
scope, operation, and unified signatures are respectively
`6022910be5471fcf`, `45fdebebd34b65b3`, and `bf12e381259515f3`.
The separate preparation harness constructs the public normal plan and
`PreparedSimulationNormalArena` for all 2,135 arenas: all 9,045 raw-derived
operations survive preparation exactly once, with no rejection or count drift.

The in-repository optimized `--inventory-bam-sim` gate then registers and
deregisters all 2,135 prepared arena occurrences with userdata `8`. Its
instrumented concrete Environment seam places kind 10 into the same event
stream as every other manager route. It compares phase, kind, full ID, waypoint
polarity, and pointer-backed or Environment backing-record identity against
each next global plan position. Environment identity is the retained arena
owner plus resource index and absolute inline-record start, so repeated TOC or
dictionary aliases of one `tInstance` compare equal while distinct physical
records do not. All 9,045 operations match in each direction. For every
arena it also compares the exact ordered `(operation index, root ordinal,
collision-root index, mesh index)` publication snapshot and requires an empty
pool after teardown. The plan-derived raw descriptor stream reproduces the
independent oracle's three frozen signatures. There are zero sequence, source,
or ground mismatches and zero surviving registrations, orphan owners, or
faults.

The corpus gate uses an empty externally seeded unlock-context table, so its
kind-10 result proves routing and ground ownership, not the still-unknown
career/location context producer or a real save-state admission decision.
The shipped corpus identities, counts, ordering, and callback/source matches
are exact. The host oracle encodings and successful-allocation assumptions are
regression policy.

## Remaining boundaries

The native processed vector stops at its first null pointer. The current dense
plan represents only successful non-null processor output, so native allocation
failure and first-null termination remain explicit producer work. The
`PreventNavmeshLoadingHack` whole-arena bypass is likewise a producer-time input,
not a kind-23 runtime branch. Location kind 21 remains a separate post-Simulation
per-arena pass, and kind 31 remains the restarted collection pass. Concrete
grind, DMO, trigger, WorldPainter, NavMesh, RAIN, AI-path, waypoint, and
SpatialMap manager implementations are still behind dispatch seams.
