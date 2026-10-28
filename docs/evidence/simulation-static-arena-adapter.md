# Supported static Simulation arena adapter

## Scope

`SimulationStaticArenaRegistrationAdapter` reconstructs the ordered arena
dispatch only for the two kinds currently represented by
`SimulationStaticArenaPlan`: kind 6 grind spline data and kind 10 Environment
instances. It does not claim to be complete `Simulation::RegArena` or
`DRegArena`.

The reference inputs are:

- XEX SHA-256
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`;
- PDB SHA-256
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`;
- MAP SHA-256
  `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`;
- external raw/decompiler packet SHA-256
  `A9D58D47623F59C478AE220859EE3F7A0333DBC1887C580AC41D222178B60207`.

The packet is external at
`world_probe_temp\sim_adapter.txt`; no executable, symbols, or
proprietary game data are stored in the repository.

## Native list and traversal

PDB types establish an eight-byte `cAssetID`, four-byte `cAssetType`, and
16-byte `cAsset` with full ID at `+0x00`, raw kind at `+0x08`, and data pointer
at `+0x0c`. The 16-byte `cAssetList` is an EASTL vector with begin at `+0x00`
and end at `+0x04`; the 24-byte `cArena` embeds it at zero.

`Simulation::RegArena` (`0x828a01e0`) and `DRegArena` (`0x828a05f0`) both:

1. compare the current pointer with end;
2. stop at end;
3. load the pointed-to asset and stop at the first null;
4. dispatch synchronously;
5. advance by one four-byte pointer.

Both directions are forward. Deregistration is not reversed. The current
dense Rust plan contains no nullable asset slots, so the adapter represents an
exact supported subsequence only for corpora, including the selected Rob's
Park set, whose builders returned non-null assets. It does not synthesize or
compress native null control flow. `ObjectId(0)` and all-ones GUID values are
ordinary data, never terminators.

## Supported dispatch

Kind 6 registration calls `GrindData::AddSplineData` (`0x82cfd228`) with the
full `cAssetID` and spline-data pointer. Deregistration calls
`RemoveSplineData` (`0x82cfb980`) with the full ID. Kind 10 passes only the
instance pointer to `UnlockableLocationMan::AddEnviroInstance`
(`0x828b02d8`) or `RemEnviroInstance` (`0x828b0188`). The downstream unlock
lookup uses the instance local GUID at `+0x68`; the processed asset's primary
GUID at `+0x60` is a distinct identity.

Both native functions return `void`, aggregate no status, and perform no
rollback. The Rust grind collaborator likewise exposes no invented result.
The Environment registration route retains its typed ground-capacity error as
safe host policy. (`StaleSource` belongs to the router's separate reevaluate,
flush, and deinitialize paths, not this adapter call.) On capacity failure the
adapter stops immediately, permanently faults its domain, and returns a
non-cloneable occurrence token identifying the processed prefix. The failing
Environment operation is included because a found context stores its pointer
and in-world state before lower host capacity can fail.

There is deliberately no partial cleanup API. If an earlier occurrence
published the same source pointer, the failed add created no new lower slot and
unconditional removal could erase the earlier occurrence instead. Restoring a
found context can also require its overwritten prior state. With no recovered
native rollback contract, every later registration or deregistration on that
domain rejects without effects; dropping the adapter is the explicit host
abandonment boundary. This fail-stop rule is host policy, not a native failure
claim.

Every complete occurrence and its teardown preserve operation order. Repeated
registration calls create repeated effects because `SimThreadMsgMan` has no
Presentation-style arena refcounter. Suppressed Environment registration is
still retained in the occurrence, and teardown always invokes its
unconditional downstream removal. Ordinary Rust `Drop` performs no hidden
deregistration.

`PreparedSimulationStaticArena` prevalidates every grind index and constructs
all Environment sources before effects begin. The concrete registration
adapter owns the grind and Environment authorities plus a live ledger that
strongly retains the complete plan and sources. Tokens carry a weak domain
identity and a checked, non-reused occurrence ID, so a foreign adapter or a
faulted domain rejects the token without effects and returns it to the caller.
Dropping a token does not remove the ledger entry; the adapter's outstanding
count and fault state expose host abandonment. These tokens, preflight errors,
and ledger rules are **high** safe-host ownership translations; the native
order, identities, calls, repetition, and forward teardown are **exact**.

## Complete dispatch boundary

The normal native switch also handles kinds 11, 16, 22, 23, 25, 27, 28, and
32. Collection registration performs a complete forward normal pass with
`LocationManager` after Simulation for each arena, then a second complete DMO
pass. Deregistration performs the full DMO pass first and then the forward
normal Simulation/Location pass. Direct arena messages and collection
messages remain separate FIFO lifecycle boundaries.

An independent selected-corpus census predicts 316 processed records:

| Source class | Count | Kind |
|---|---:|---:|
| Spline | 42 | 6 |
| Location | 4 | 21 |
| Environment instance | 26 | 10 |
| WorldPainter | 145 | 22 |
| RAIN | 4 | 25 |
| AI path | 4 | 27 |
| Waypoint | 3 | 28 |
| DMO | 43 | 31 |
| SpatialMap | 45 | 32 |

The static plan contains the exact ordered 68-record `{6,10}` subsequence. The
separate `SimulationNormalArenaPlan` now merges it with the 201 kind-22/25/27/
28/32 records in native TOC order and also models zero-count kind 23 for this
selection. Those ID-only records are planning data, not live manager effects.
Kind 31 belongs to the separate DMO pass and kind 21 belongs to
`LocationManager`; neither is silently treated as registered.

## Real-data validation

The optimized `--probe-robs-park-resident` run passed over all four selected
Simulation collections and 68 resident arenas. Both its direct and
parent-expanded compositions reproduce 42 grind plus 26 Environment
operations and ordered plan signature `39413e2d647f8631`. The adapter records
68 successful forward registration operations, keeps 26 identity-placed
Environment roots live, then records 68 forward deregistration operations,
requires the 42 grind remove IDs to equal the add sequence, and ends with zero
roots.

The app deliberately invokes the static adapter during diagnostic asset
build/`Drop`. It
proves supported operation order, ownership, and teardown against real data;
it does not prove native message timing, static GrindData storage/query
visibility, unlock-context production, Location/DMO passes, or complete world
activation.

Focused tests cover empty lists, mixed kind order, full-width grind IDs,
forward removal, found- and absent-context partial failures, an explicit
all-ones local-GUID context, unlock-suppressed teardown, repeated uncoalesced
occurrences, invalid preflight indexes, foreign-domain rejection with token
recovery, prepared-source lifetime, and absence of implicit `Drop` cleanup.
Two regressions prove that a second same-source capacity failure and two
same-root operations within one occurrence fault without removing the valid
earlier publication.
