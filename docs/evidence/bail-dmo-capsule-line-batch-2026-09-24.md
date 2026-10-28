# LivingWorld DMO capsule on the retained PhysicsWorld line batch

Scope: selected offline simulation-island World with its real initialized DMO
collision collection empty. Confidence **high** for branch and owner order,
**medium** for host floating-point parity. The native populated collision
collection and visual-island override are not admitted by this narrow route.

`Sk8::LivingWorld::Manager::DoesAnyDynamicObjectIntersectCapsule`
`0x82703930..0x82703a58` reads the Simulation singleton and `mPhysicsWorld`
(`+0xc8`) and asserts either missing occurrence. It calls World virtual
`AcquireLineBatch` at `0x827039b4` with label
`"LWMan::DynamicObjectTest"` and `BF_Dmo=2`. It then calls the returned
batch's virtual Begin `+4`, AddTest `+0x1c` with both input Vector3 pointers
and `radius.mV.x`, End `+8`, Sync `+0xc`, GetResult(0) `+0x20`, and Release
`+0x10`, in that exact order. The return is true only when the result pointer
is nonnull; Release occurs before that bool is formed.

`World::AcquireLineBatch` `0x827ab918..0x827ab94c` passes the requested flag
to its simulation island except when its separate `mVisualIslandEnabled` bit
is set, in which case it substitutes `BF_Default=7`. The selected host World
currently constructs only a simulation island, so it retains the flag-2 path.
`Island::AcquireLineBatch` `0x827b8728..0x827b885c` acquires the actual DMO
pool line batch as the sole child for flag 2. The virtual AddTest resolves to
`ClusteredMeshLineBatch::AddTest` `0x827bd9a0..0x827bda68`, which stores the
full start and end vectors and the scalar fatness. The native function name in
LivingWorld calls this a capsule; it is **not** a separate capsule batch
family. `ClusteredMeshLineBatch::GetResult` `0x827bda68..0x827bda9c` returns
null iff the executed result's hit count is zero.

The retained Rust owners already implemented this same line batch and the
`CollisionLineInput` start/end/fatness representation. A new narrow
`SimulationWorldLineQueries::try_dmo_capsule_test` routes through the existing
acquire/begin/add/end/sync/get/release implementation with exact label and flag
2, then converts the released result to bool. It queries the same DMO
collection as other World line tests. `StaticGroundPool` is passed as the
World's existing ground collaborator to the shared End method, but no ground
child is acquired or visited for flag 2. An actually empty DMO pool and empty
constructed collision collection can yield a real miss; a populated DMO
membership refuses at End because its collision publication route is not yet
recovered. This refusal retains the in-progress lease rather than pretending
the request missed.

Focused tests in `simulation_world_line_query_tests.rs` check two successive
constructor-empty misses (thus Release), radius retention, only DMO count
advancing, and a real inserted DMO producing the typed unpublished-membership
error with its Running lease retained. The root integrator owns Cargo runs;
only local `rustfmt` ran in this delegated change. The bridge to the
LivingWorld Manager and breadcrumb callback remains for integration.
