# Island-indirect Horus pass schedule

Date: 2026-09-05 (Europe/Helsinki)

This checkpoint reconstructs one invocation of
`rw::physics::HorusPipelineIslandUsingIndirectJacobians` at
`0x82ae2858..0x82ae3190`. It connects the already checked Contact, Joint, and
Drive selected-island consumers to their shared live rigid-body RF storage. It
does not infer an order between separate island jobs and does not yet own the
all-island barrier or subsequent `RigidBody::DynamicUpdate` call.

## Provenance

The authoritative packet is
`world_probe_temp\horus_outer_schedule_evidence.md`, SHA-256
`EA078BDFBD4DFE02517E8A9574AA481EAF6AAF9E643C21684D50972C45D773B0`.
Its normalized raw instruction evidence and extractor hash to
`E902A46A2588C31EFCF045AAA1C75A46D5C89E6DAAF0C5F3AE220D2AB3D3020E`
and
`76CB28ED4D8DC307EFDB6045BF8305FE168810C09D7438D5607C3B4E22704AE1`.
The independent schedule oracle source/output hashes are
`3C98F7ACD0DF5D398959E1D9A70BDE541C129B7481CB2106CFA58B409484F754`
and
`D99A4E11D35F9C0B8F3D3BFC83DF8BA7373A6F7407820CD3FB8F75A1750099D3`.

## Recovered behavior

The native function snapshots the selected island's Contact, Joint, and Drive
count/list headers and the unsigned runtime `Simulation::m_MaxIteration` value.
The shipped World initialization writes 25, but this solver does not embed 25.
After its zero gate, each pass consumes the complete Contact list, then the
complete Joint list, then the complete Drive list. Lists retain their supplied
order and repetitions, records retain accumulator state between passes, and
every visit reads then-live RF rows.

The existing family consumers preserve their separately proven effects:

- Contact mutates its packed accumulator and commits
  `A3,B3,A1,B1,A2,B2,A0,B0`.
- Joint mutates Angular then Position accumulators and commits `A0,B0,A2,B2`.
- Island Drive mutates Position then Angular accumulators and commits
  `A0,B0,A2,B2`.

The host first validates and allocates the three complete selected-island plans
before pass zero. One transaction then owns the exact mutable record slices and
the exclusive DynamicUpdate phase for the entire runtime-counted loop. This
pre-effect failure atomicity and occurrence-qualified index retention are safe
host ownership policy; the native function assumes valid pointer tables.

The host validates the fixed `0..10` island range before the zero gate. A valid
island with zero iterations does not inspect any selected record/body
occurrence. For nonzero execution, recoverable Contact, Joint, or Drive
preflight errors leave all three record families and RF storage unchanged.
After execution begins, a panic is deliberately nontransactional: earlier
effects and mutation of the currently entered record remain, while stores not
yet reached and later family work do not occur.

## Verification

A real two-island integration fixture creates nonempty compiled Contact, Joint,
and Drive tables. Running island 2 for two passes through the combined entry is
bit-identical across every record word and RF row to the literal sequence
`C,J,D,C,J,D`. The same fixture differs from a grouped
`C,C,J,J,D,D` mutant, proving that the comparison is schedule-sensitive. Every
selected family record changes, while the island-7 records and exclusive RF
rows remain bit-identical.

Additional tests prove that:

- island 10 is rejected even when the runtime count is zero;
- a valid zero-count call bypasses a deliberately wrong Drive authority;
- the same wrong authority reaches the later Drive preflight for a nonzero
  call and leaves already prepared Contact and Joint state untouched;
- two-pass repeated plans preserve exact Contact -> Joint -> Drive live-RF
  chronology; and
- a pass-one Joint panic retains the precise cross-family prefix, including the
  current Joint record mutation but excluding its RF stores and all later Drive
  work.

The audited implementation source hashes are:

| Source | SHA-256 |
| --- | --- |
| `simulation_horus_pipeline.rs` | `609682B383DACE2DA119F57CDB6EFE01BDA1FA6462744239EFEFA0F1141F304C` |
| `simulation_rigid_body_pool.rs` | `278FA59905B1E41F37128D258FA8C42A4A24AC8C40FCD8A7A1AE2D345F5FA47C` |
| `simulation_contact_jacobian.rs` | `0AFCF638D24D850204B82D3E28D903750445446AD43091EB93A806483351CC58` |
| `simulation_joint_preparation.rs` | `70B15A40EA77FB49EE756C5D82F463476B0B5C08F8C9F6B80A8AE2BB787F3398` |
| `simulation_drive_jacobian.rs` | `6B51406CBBE7287555B845054ACAEB611E2D55F1CBF3580AB3DDB8CC05E00156` |

## Remaining boundary at this checkpoint

This is one private, prepared, network-Contact island invocation. The native job
graph establishes that every island job follows all three compiler jobs and
that DynamicUpdate follows every island, but it establishes no order between
island jobs. Cross-island RF disjointness/serialization, all-island scheduling,
the barrier, phase consumption into DynamicUpdate, non-network Contact
producers, native volatile publication, and exceptional Xenon floating-point
parity remain open.

# Prepared all-island serial barrier and DynamicUpdate

Date: 2026-09-05 (Europe/Helsinki)

This later checkpoint adds one private owner for a complete prepared island
solve followed by the recovered ordinary-finite DynamicUpdate body sweep. It
does not revise the one-island result above. The selected-island C -> J -> D
consumer remains the exact inner operation; this checkpoint supplies an
explicit deterministic PC policy for the native cross-island scheduling gap.

## Native job graph and invocation ownership

`EA::Physics::SimulationManager::PopulateJobTree` at
`0x82e0b410..0x82e0b67c` first reads `Simulation::m_ActiveRB_Count` at
`0x82e0b430..0x82e0b43c`. If it is zero, the routine adds only the dependency
from the end node to the start node at `0x82e0b440..0x82e0b444` and returns.
No Contact, Joint, or Drive compiler, island solver, or DynamicUpdate job is
created on that route.

For a nonempty Active list, Contact, Joint, and Drive compiler jobs are prepared
in source order at `0x82e0b450..0x82e0b49c`. Each depends on the same start
node, so they are siblings; their preparation order is not an execution-order
edge. With islands enabled, native sign-extends manager `mMaxIslands` and
constructs jobs for indices `0,1,...,count-1` at
`0x82e0b4ac..0x82e0b510`. Every island job depends on the Drive, Joint, and
Contact compiler jobs at `0x82e0b4d4..0x82e0b504`. There is no dependency
between island jobs, so ascending construction does not prove ascending,
serial, deterministic, or nonoverlapping runtime execution.

The per-island manager job parameter stores the shared `Simulation *` and its
unsigned island index at `0x82e0b170..0x82e0b180` and `0x82e0b1b8`. The island
job thunk reloads the index and shared Simulation, then tail-calls
`HorusPipelineIslandUsingIndirectJacobians` at `0x82e0b070..0x82e0b078`.
DynamicUpdate is prepared at `0x82e0b558..0x82e0b568` and receives one
dependency on every island job at `0x82e0b574..0x82e0b5a0`. Its thunk calls
`Simulation::BatchIntegrator` at `0x82e0b088..0x82e0b08c`. This proves a full
all-island completion barrier before DynamicUpdate, not an order among the
islands themselves.

`Island::Step_Collision3` constructs this graph by calling `PopulateJobTree` at
`0x827b40e4`. `Island::Step_Solver2` later performs its
optional thaw/partition work, synchronously calls
`CreateSimulationIslandTables(0)` at `0x827b369c..0x827b36a8`, and only then
submits the end-rooted job tree at `0x827b36ac..0x827b36b4`. The table build is
therefore complete before the compiler and solver jobs can run on this observed
route.

After the barrier, `Simulation::BatchIntegrator` at
`0x82ae4a40..0x82ae4a84` starts at the Active anchor's right link and follows
the right ring, calling `RigidBody::DynamicUpdate` once per Active body at
`0x82ae4a54..0x82ae4a6c`. It is a raw do/while without an internal empty-ring
guard; the manager's earlier Active-count branch supplies that guard.
`RigidBody::DynamicUpdate` itself is `0x82ae4488..0x82ae4904`. Its terminal
stores clear RF rows `d3,d2,d1,d0` at `0x82ae48d0..0x82ae48dc`, restore force
from gravity at `0x82ae48e0..0x82ae48f0`, and clear torque at
`0x82ae48f4..0x82ae48fc`.

## Explicit deterministic host policy

The PC owner validates an active route's configured island count as `1..=10`,
prepares all scheduled work before the first solver effect, executes islands
serially in ascending construction-index order, and consumes DynamicUpdate
once after the last island. The range is the fixed native representation and
the observed public gameplay routes supply only positive counts 1, 8, or 10;
native itself has no recoverable range check. Both the rejection and the
ascending serial schedule are therefore safe deterministic host policies, not
claims about native error handling or scheduler order.

The owner also uses one frame-wide `u32` maximum-iteration snapshot. Native
loads `Simulation::m_MaxIteration` independently at each island-function entry
at `0x82ae2864`; no recovered edge in this packet orders or freezes those loads
across separate jobs. Reusing one stable value is deliberate host frame
ownership, while the exact per-island unsigned pass count and C -> J -> D
ordering remain recovered behavior.

For a positive iteration count, Contact, Joint, and Drive plans for every
scheduled island are validated and fully allocated before island zero begins.
Native validates none of these pointers and could already have effects before a
later invalid access. Whole-frame, all-family preflight is a stronger host
all-or-error boundary. Once execution starts, there is no returned error,
sorting, deduplication, endpoint remapping, or late allocation between island
effects.

Serial execution is necessary at this checkpoint because record-table
partitioning does not prove RF-row partitioning. Each constraint record is
appended to one selected island, but Joint and Drive do not compare valid A/B
labels and select B unless B is exactly `-1` at `0x82ae3f78..0x82ae3f88` and
`0x82ae4074..0x82ae4084`; Contact selects A unless A is exactly `-1` at
`0x82ae3e80..0x82ae3e90`. Every solver still reads and writes both endpoint RF
rows: Contact's eight stores span `0x82ae2be8..0x82ae2c2c`, Joint's four stores
span `0x82ae2ea0..0x82ae2ee4`, and Drive's four stores span
`0x82ae3134..0x82ae3178`.

Contact is stricter still: its precompile table-body roles and compiled RF roles
are independent bindings; the in-place compiler stores RF `ia/ib` at
`0x82ae1ae8` and `0x82ae1c30` without proving them equal to the table bodies.
The recovered network Contact mapping is
`[table A, table B, RF 0, RF 1] = [static, selected, static, selected]`, so
records routed by bodies in different islands may retain the same manager
Static RF identity; the relevant network producer span is
`0x827b3c7c..0x827b3ec4`. The host preserves shared, self, and repeated
endpoints in serial order rather than rejecting or parallelizing them. No
current claim says that native island jobs actually ran serially or that
concurrent Static writes were disjoint or numerically benign.

## Zero-active and zero-iteration routes

The two zero cases are observably different:

- An empty prepared Active phase follows the native Active-count gate before
  host island-count validation. It performs no family plan preparation, solver
  callback, body integration, or RF clear. The private phase is simply dropped.
- A nonempty Active phase with maximum iterations zero still crosses the
  all-island barrier and consumes DynamicUpdate once. Native island entry
  snapshots all six Contact/Joint/Drive count and list-base header words before
  branching at `0x82ae2860..0x82ae28c8`, but dereferences no table entry and
  performs no constraint-record or RF solver mutation. Table construction and
  the three compiler jobs are not suppressed by the iteration value. The host
  correspondingly skips per-entry family traversal and revalidation, then
  integrates the already-preflighted Active plan. This matters because RF
  lifetime crosses StartFrame and DynamicUpdate may consume and clear values
  that predate the zero-pass solve.

## DynamicUpdate phase ownership and failure boundary

The outer entry takes an already-prepared
`SimulationRigidBodyDynamicUpdatePhase` by value. Phase preparation validates
all four retained body rings and RF-row cardinality, reserves the complete
Active-order plan, requires every Active body's Inertia, copies the right-ring
indices and Inertia values, and freezes timestep, frequency, gravity, cooldown,
and minimum-energy configuration. It also binds the body-pool frame and a
private nonreused phase identity. Its exclusive pool borrow prevents topology,
occurrence, Inertia, configuration, and RF-storage replacement while family
plans and solvers use the phase's restricted interfaces.

Every returned phase-preparation or family-plan error occurs before a record,
RF, or body effect. If family preparation fails after the phase was issued,
dropping it performs no integration or RF clear, but the private phase identity
remains consumed. That identity burn is safe-host replay protection, not a
native field or a whole-frame rollback violation. Phase integrity validation
currently uses ordinary host allocation for its temporary seen-set; a
catastrophic allocator abort there is not represented as a typed preparation
error, although it still precedes all solver effects.

Successful consumption has no returned error, allocation, validation, lock,
occurrence lookup, or caller-supplied callback. It moves the copied Active plan
in right-link order and calls the closed ordinary-finite DynamicUpdate graph.
The current numeric function has no normal data-driven panic path. The contract
nevertheless remains nontransactional under an internal or future numeric
panic: earlier bodies and the current body's stores already reached remain,
and no rollback occurs. A solver panic similarly retains the exact earlier and
current constraint/RF prefix; unwinding drops the phase inertly, so
DynamicUpdate is skipped rather than run implicitly.

## Verification and remaining discriminators

The integrated acceptance fixture now distinguishes these cases with real
compiled network Contact, Joint, and Drive records on islands 2 and 7:

1. `combined_horus_serial_empty_active_bypasses_invalid_count_and_all_work`
   gives the owner nonempty prepared Contact data, a foreign Drive pool, and an
   invalid zero island count under an empty Active phase. It returns without
   changing any record, RF row, or body.
2. `combined_horus_serial_active_route_validates_host_island_count_before_effects`
   proves that a nonempty phase rejects island counts 0 and 11 before every
   record, RF, or body effect. The same real fixture accepts observed public
   route values 1, 8, and 10.
3. `combined_horus_serial_zero_iterations_skips_poisoned_drive_and_integrates_once`
   proves that maximum iterations zero bypasses the foreign Drive authority and
   all family records, exactly matches an integrate-only oracle, changes the
   Active bodies, and clears every seeded Active RF row.
4. `combined_horus_serial_owner_rejects_later_island_before_every_effect`
   corrupts Contact island 7 after all batches exist. It receives the exact
   island-7/table-ordinal-zero refusal and leaves both islands' records, every
   RF row, and every normalized body word unchanged. The retained Active RF is
   explicitly nonzero, so equality cannot hide an accidental integration.
5. `combined_horus_serial_owner_matches_literal_ascending_islands_and_integrates_once`
   compares every Contact/Joint/Drive word, every RF word, and every normalized
   body word against `for island in 0..8 { C,J,D,C,J,D }`, with nonempty work at
   islands 2 and 7. Both real island records change, DynamicUpdate changes
   Active bodies, and every Active RF row is zero afterward.

The counted RF-plan tests additionally freeze allocation-before-iterator
behavior, short and trailing iterator refusal, later stale occurrence
atomicity, exact record-slice ownership, live-RF range reuse, phase mismatch
before even an empty range, and empty/nonempty Active reporting. Existing
DynamicUpdate tests independently prove Active right-ring rather than slot
order, Static/Frozen exclusion, copied Inertia/configuration use, exact terminal
RF clears, and inert phase drop. The existing one-island panic test freezes a
solver-prefix unwind, but an all-island later-island panic discriminator and a
test-only integration panic injector remain optional hardening rather than new
recovered behavior.

The audited implementation source hashes for this checkpoint are:

| Source | SHA-256 |
| --- | --- |
| `simulation_horus_pipeline.rs` | `AD05096526E315CBD4292B01219746C78042F303E728B4E9F58AA33ED27E570B` |
| `simulation_rigid_body_pool.rs` | `F0D5DDA192606564E068E826AA19B717991FAEE7CEEAE22836624DA4DBF53B0E` |
| `simulation_contact_jacobian.rs` | `705E4673CBE8817FC0AEE9A81258F7AAB40DD41273732ED61D39F0B258FDBEDA` |
| `simulation_joint_preparation.rs` | `2658195783269AE0D31854DD959C55EE102BD3AD11230DB06D167062881F927C` |
| `simulation_drive_jacobian.rs` | `A5D4F43803AF4D681E2A8756FDE84F0A69ECEEAC26C229642E2B9709723A9FBC` |

## Remaining production and parity boundaries

The new owner is still crate-private prepared infrastructure, not a gameplay
World/Island call-site integration. The current Contact family is the sealed
network producer; the non-network collision producers and their complete
occurrence binding remain open. Native scheduler worker count, fairness,
cross-island wall-clock order, interleaving, and any special handling of shared
Static RF are unproved. A future parallel backend requires a separately proven
RF-ownership or conflict policy and cannot infer disjointness from island body
tables.

The non-observed all-spy-bits-zero end-node route, native volatile Contact-count
publication, live runtime writers of `m_MaxIteration` or DynamicUpdate
configuration, and exact Xenon estimate/reduction, denormal, NaN, infinity,
signed-zero, and FPSCR behavior remain outside this checkpoint. This section
adds no new source-hash claim; final implementation hashes belong to the later
acceptance record.
