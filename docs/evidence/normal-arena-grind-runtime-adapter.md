# Source-backed grind callbacks in normal arena registration

Confidence: **high** for callback placement, source identity and checked failure
ordering; combined Cargo and original resident-session validation are pending.

Fresh raw `Simulation::RegArena828a01e0` proves the kind6 body at828a037c
loads the full64 GUID from the processed record, loads its spline pointer
at+C at0380, then calls `GrindData::AddSplineData82cfd228` at0384. The
normal vector walk continues forward afterwards. `DRegArena828a05f0` loads
only that GUID at078c and calls `RemoveSplineData82cfb980` at0790; it does
not reread or parse the spline payload. Neither kind6 body reads the kind11
userdata bit. Null processed-asset termination, omitted processors, the
separate Location and DMO collection passes, and complete native asset-manager
timing remain outside the existing represented normal-arena contract.

The new child `simulation_normal_arena_grind_adapter.rs` reuses the existing
normal arena loop and live/fault ledger. `PreparedNormalGrindArena` performs
the existing normal source validation, then caches a `GrindSplineSource` for
each actual kind6 operation using the same `Arc<SimulationNormalArenaPlan>`
and global normal-operation index. This is checked-host preparation of the
already-retained processed arena, not an assertion that native RegArena
parses serialized source data. Sources retain original full TOC GUIDs and
the actual spline occurrence. Non-grind operations have no invented source.

`SimulationNormalGrindArenaAdapter` retains the supplied actual world-owned
`Arc<Mutex<GrindDataRuntime>>`; it does not initialize or replace that owner.
At each reached kind6 callback it acquires that owner, then Add constructs the
actual source record using `HostGrindDataNumeric`. Source numeric exceptions
remain the explicit numeric policy described by the runtime module. The lock
is acquired only at the kind6 operation, after earlier non-grind callbacks.
DReg obtains the cached GUID and calls Remove without parsing, constructing an
octree, or evaluating numeric functions. Successful repeated registration
retains distinct occurrences; native Remove erases every matching full GUID
using its swap-removal semantics, even across those occurrences.

The narrow parent change adds an explicit fallible grind callback beside the
existing DMO tail callback. Existing public legacy APIs explicitly supply the
same old `SimulationStaticGrindDispatch` calls. The new wrapper supplies the
actual owner callback exclusively. There is no optional-provider fallback,
post-registration replay, separate kind6 pass, world-store snapshot walk or
legacy diagnostic invocation on the actual route. A distinct token contains
the underlying registration authority and cached source bundle; the base
adapter remains private so ordinary use cannot route that token through the
legacy void API.

Safe-host source/owner/runtime failures latch the actual current operation and
`Grind` stage with the copyable `GrindRefusal` marker, while the concrete error
is returned by the typed wrapper. Earlier manager effects and the live plan
owner remain retained, the suffix does not run, no rollback occurs, and the
domain remains faulted. An actual callback panic retains the existing
`DispatchPanic` behavior. Before-effect deregistration errors return recoverable
authority through `into_registration`, reconstructed with its same source
bundle. A reached grind refusal consumes the token and offers no teardown of
the partially processed occurrence. Token Drop dispatches no hidden cleanup.

Four new focused tests reuse the existing normal-arena fixture infrastructure:
actual Add/DReg with duplicate full GUIDs and no legacy grind calls; a later
empty-source AssetRecord refusal after an earlier valid grind and intervening
registry callback; foreign-route token recovery without effects; and poisoned
owner refusal reached after the first deregistration registry callback, with
the live ledger retained and no recoverable token. They are registered under
the existing test module so private diagnostic fixtures need no public API.
The implementation agent did not run Cargo; root owns the combined gate and
original resident-session integration.
