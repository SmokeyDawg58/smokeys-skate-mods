# Actor-owned graph constructor integration — 2026-09-11

The same retained Actor now constructs and owns ActionGraph followed by
SkaterMotionGraph after its listener is published. This advances the selected
constructor to `GraphFacadesConstructed`; it does not load or execute graphs,
construct animation/physics, publish a complete Actor, or provide playable skating.

## Source and implementation

Fresh IDA raw instructions confirm Actor `825fec5c..825fecc8`: Action allocation,
constructor `82cea340`, member `+1f4` publication, then Motion allocation,
constructor `825fdda8`, derived interface `base+a78` publication at `+1f8`.
Outer null allocations publish null and still continue. Wrapper allocation-null
inside either constructor also completes that constructor with a null wrapper.

`actor_graph_construction.rs` owns the actual empty graph components, three
capacity-64 intent stores, capacity-20 motion attribute store, distinct wrapper
controller identities and the same Actor Entity issuance. Wrappers have root
hash zero and null abstract controllers. `actor_construction.rs` registers graph
interfaces in its existing sorted Entity registry, retaining component occurrences
and checking duplicate identities. Base and derived Motion interfaces designate
the same component, with distinct interface kinds. Actor member publication
follows complete constructors; late failures retain registered prefixes and
partial allocations and cannot be replayed.

Original constructor/reset byte hashes match
[the source packet](actor-graph-constructor-runtime-plan.md): Action `82cea340`,
Motion `82cea060`, derived `825fdda8`, Reset `825f6110`. Fresh vtable reads resolve
Reset calls to `ClearMgAttributes 825fe1d0` and transition reset `825fdf50`.
Unknown low transition bits remain unknown; the constructor/reset packed state
retains its known low bit. Original six Random words were read at `8220554c`.
Runtime Vault tuning is not replaced with constants by this change.

## Verification and limits

Root independently reviewed the graph author's raw source and implementation.
A separate review approved root's Actor registry/mounting and tests;
the requested corpus cleanup correction keeps the listener registered throughout
graph construction and unregisters it afterward.

Focused `cargo test -p skate2-core --lib actor_`: **25 passed**, zero failures
(2026-09-11; tool checkpoint `9c9db8`). This filter also includes existing tests
whose names contain `actor_`. The changed original-data corpus and strict workspace
checkpoint remain pending until the current integration batch finishes.

Host owned allocations and checked Vec reservations are explicit implementation
policy, not native allocator/category parity. Intent insertion/bucket traversal,
generated execution, subsequent Actor construction and deterministic reference
tick comparison remain unfinished. The corpus still uses controlled input samples,
cvars, flush timing and a diagnostic intent sink; constructing graph storage does
not by itself connect that sink to running gameplay.
