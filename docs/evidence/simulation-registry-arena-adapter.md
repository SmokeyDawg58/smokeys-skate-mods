# ID-dispatched Simulation arena adapter

## Scope and provenance

`SimulationRegistryArenaRegistrationAdapter` is the bounded runtime seam for
normal Simulation kinds 22, 23, 25, 27, 28, and 32. It consumes the matching
operations from an owner-bound `SimulationNormalArenaPlan`, forwards only the
full 64-bit processed asset ID, and walks the filtered subsequence forward in
both directions. It is not a complete `Simulation::RegArena`: kinds 6, 10, 11,
and 16 remain in their existing adapters, and running those adapters as
separate passes cannot reproduce global cross-kind interleaving.

Reference inputs are:

- XEX SHA-256
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`;
- PDB SHA-256
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`;
- MAP SHA-256
  `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`;
- dispatch packet SHA-256
  `A9D58D47623F59C478AE220859EE3F7A0333DBC1887C580AC41D222178B60207`.

The executable, symbols, decompiler database, packet, and game corpus remain
external to the repository.

## Exact dispatch boundary

`Simulation::RegArena` at `0x828a01e0` and `DRegArena` at `0x828a05f0`
traverse the processed asset-pointer vector forward and terminate at the end
or first null pointer. The MAP signature and caller `lwz` instructions prove
that userdata is a full `u32`, despite a decompiler rendering it as `char`.
Only kind 11 reads it. The six routes represented here do not sample userdata:

| Kind | Registration | Deregistration |
|---:|---|---|
| 22 | `WorldPainter::LayerMan::RegisterStreamedTile` `0x82ceb770` | `UnRegisterStreamedTile` `0x82ceb268` |
| 23 | `LivingWorld::MeshMan::AddNavMeshData` `0x82ce6a80` | `RemoveNavMeshData` `0x82ce6ac0` |
| 25 | `LivingWorld::RAINMan::AddRAINData` `0x827202d8` | `RemoveRAINData` `0x82727510` |
| 27 | `AIPath::PathManager::AddAsset` `0x824d4890` | `RemoveAsset` `0x824d0f78` |
| 28 | `LivingWorld::Manager::RegisterWaypointData(id, true)` `0x82719e60` | `DeRegisterWaypointData(id, false)` `0x82722088` |
| 32 | `SpatialInstanceManager::AddAsset` `0x82dfe480` | `RemoveAsset` `0x82dfe3f8` |

All calls receive the complete `assman::cAssetID`; the surrounding `ld`
instructions disprove split-low-word decompiler artifacts. All return `void`,
there is no local rollback/status channel, and deregistration is forward rather
than reversed. Once kind 23 reaches this switch it is unconditional. The
`PreventNavmeshLoadingHack` test belongs to producer-time `ProcessArena` at
`0x82d60238..0x82d602e8`; it is not a runtime kind-23 gate.

The adapter retains one complete prepared plan per explicit occurrence before
the first manager call, assigns a non-cloneable same-domain token, preserves
duplicates, and consumes that token only after forward deregistration.
Foreign or unknown tokens produce no effects and are returned to the caller.
Its dispatcher field precedes the live source ledger so manager state is
destroyed before any arena bytes it may reference. Token and adapter `Drop`
make no manager calls. Checked storage/ID errors are safe host policy and occur
before effects; collaborator panics and deep manager failure behavior remain
outside this void seam.

The deep WorldPainter, NavMesh, RAIN, AI-path, waypoint, and SpatialMap manager
implementations are not claimed here. They will replace the dispatch recorder
incrementally as their own evidence packets and corpus tests are completed.

## Whole-BAM runtime gate

The optimized `--inventory-bam-sim` route prepares, registers, and deregisters
the ID-only subsequence for all 284 BAM Simulation collections and 2,135
successfully decoded arenas. It observes:

| Kind | Operations |
|---:|---:|
| 22 WorldPainter | 6,122 |
| 23 NavMesh | 0 |
| 25 RAIN | 75 |
| 27 AI path | 101 |
| 28 waypoint | 95 |
| 32 SpatialMap | 1,033 |
| **Total** | **7,426** |

Every operation is called once in each direction. The gate compares each
recorded event against its prepared descriptor, requires literal waypoint
polarity, preserves collection/XSM and arena/XSF order, and ends with zero live
occurrences. Raw-corpus reconciliation independently gives the same six counts.
Together with kinds `{6:819, 10:790, 16:10}`, the nominal whole-BAM normal
corpus contains 9,045 operations; this corpus contains no kind 11 or kind 23.

The V1 FNV-1a streams begin with distinct registration/deregistration labels,
then encode `C(filename,NUL)`, `A(outer asset u64, RW4 arena u32)`, and one
`O(kind u32, asset ID u64, waypoint byte)` per event. The waypoint byte is
`1`/`0`; other kinds use `0xff`. Numeric fields are big-endian. Frozen results
are:

- registration `757b00e47ce5f273`;
- deregistration `b55ae55479c67d26`.

The shipped IDs, counts, and authored order are exact. FNV-1a, this encoding,
checked allocation, and token authority are host regression/safety policy.
The dense successful plan cannot represent a native null processed-asset slot,
so first-null behavior remains a required producer input rather than a
fabricated terminator here.

## Unified integration status

`SimulationNormalArenaRegistrationAdapter` now prepares one globally ordered
vector covering static, pointer-backed, and ID-only operations, then traverses
it once per direction with one arena-call userdata snapshot. It does not invoke
the three filtered adapters sequentially. See
[`simulation-normal-arena-adapter.md`](simulation-normal-arena-adapter.md).
At collection level, the exact active order remains complete
`Simulation::RegArena` then `LocationManager::RegArena` for each arena, followed
by a restarted complete kind-31 DMO pass. Teardown performs the complete DMO
pass first, then Simulation and Location per arena. Outer and inner first-null
slots remain explicit producer work.
