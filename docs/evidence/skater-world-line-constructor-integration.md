# Scoped World line query with constructor-empty dynamic owners

2026-09-08. This follows the skater-first priority
and preserves the parked DMO implementation. Source inspection and scoped
implementation are recorded here; this packet does **not** claim a new Cargo
checkpoint, native frame trace, completed offline spawn, or playable controls.

## Source constructor and selected island

Direct IDA inspection confirmed these native owners and call sites:

| Native symbol/address | Relevant behavior |
|---|---|
| `World::Initialize` `827b0e58` | Samples `CommonBackEnd::sGhostSystemEnabled` into `mVisualIslandEnabled`. Selected simulation island receives `FLAG_Both` when false and `FLAG_Simulation` when true, plus the actual network manager and World ground pool. Constructor selection occurs at `827b1074..827b1090`. |
| `Island::Island` `827bab18` | Retains the actual World, flags, manager and ground; invokes `InitWorld`, followed by culling initialization. |
| `Island::InitWorld` `827b42c0` | Constructs distinct DMO and vehicle `DynamicPool` owners; calls at `827b46bc` and `827b486c`, stored at Island `+6cc8/+6ccc`. |
| `AssemblyPool::AssemblyPool` `82e0d9c0` | Zero assembly count at `82e0d9e4`, actual manager retained at `82e0da14`; allocates base membership/serialized storage from constructor capacities. |
| `DynamicPool::DynamicPool` `827caf20` | Calls base constructor, stores collection count zero and spatial pointer null at `827caff8/827caffc`. Allocates mesh capacity and constructor-resets its backing entries, while published count remains zero. |

PDB enums: `FLAG_Invalid=0`, `FLAG_Simulation=1`, `FLAG_Visual=2`,
`FLAG_Both=3`; query flags Ground=1, DMO=2, Vehicle=4, Default=7.

DMO assembly and mesh capacities are both 50 (`827b45dc/827b4618`).
Vehicle capacities are both 24 when island flags mask 1 is set, otherwise
zero (`827b4798..827b47d0`). **Both possible selected simulation-island
flags, 1 and 3, therefore construct capacity-24 vehicles.** The zero-capacity
vehicle route belongs to the other island configurations; it cannot justify
substituting a zero-capacity owner into the selected simulation island.

Each pool samples its distinct parallel-update cvar and its line, box and
tests-per-batch cvars. With flags mask 1 set, trajectory count is the sampled
skater trajectory count plus 90 and step-selector count is 18. Otherwise DMO
uses trajectory/step counts 1/1 and vehicle 0/0. Dynamic constructor batch
allocations are all inside the nonzero mesh-capacity branch at
`827cb040..827cb1ac`. The step-selector allocation uses literal tests-per-batch
one, independently of the other families' sampled test count.

The scoped host API receives constructor batch configurations. Their runtime
sampling remains the caller's responsibility; corpus values are controlled
diagnostic inputs, not evidence of a complete cvar service or live scheduling.
The dynamic query constructor explicitly supplies the source step-selector
test count one. It accepts only selected simulation flags 1 or 3.

## Query dispatch, order and result ownership

The earlier [IK line packet](skeleton-ik-line-query-source.md) records the
actual IK Island virtual slot `+44`, resolved to forwarding body `827b2538`.
It routes through that Island's World. World's label-only acquire `827b1a58`
supplies flags 7. Flags overload `827ab918` selects **World's retained
simulation island** and resets requested flags to default when visual-island
mode is enabled.

`Island::AcquireLineBatch` `827b8728..827b885c` acquires Ground first
(`827b8788`), DMO next (`827b87b8`), and vehicle (`827b87f0`) only for
`mFlags == FLAG_Both` and requested vehicle bit. Selected default requests
therefore always have two or three children and use an aggregate.

`World::LineCollisionTest` `827ab338..827ab448` performs acquisition, Begin,
AddTest, End, Sync, GetResult(0), result copy, and Release. Each aggregate
stage visits its children in retained order before the next stage starts.
Aggregate AddTest `827bbe50` visits every child and ANDs results without
short-circuiting; World does not use that boolean as a miss result.

Fresh inspection of `AggregateLineBatch::GetResult` `827bbec8..827bbf60`
confirms its per-child count guard at `827bbf08`: it **skips** a child when
the requested index is not below that child's test count. A child at capacity
that declined AddTest does not force a child GetTest index error. Among
available hits, only a strictly ordered smaller line parameter replaces the
current result (`827bbf40`); ties and unordered comparisons retain the earlier
child. Position and normal retain all four lanes; parameter, surface and owner
copy before release. Ground supplies null owner.

## Implemented scoped owners

`simulation_world_line_query.rs`, registered as ConstraintSession's
`world_line_query` child, supplies:

- `SimulationWorldLineQueries::try_new(manager, backend, dmos, island_flags,
  config)`: same-manager/backend validation, actual constructor-empty query
  collections, all four retained batch families, and a capacity-24 vehicle
  base membership owner. No source image or fabricated arena binding is
  needed for empty membership.
- `try_line_test(ground, dmos, input, label, numeric)`: actual borrowed
  `StaticGroundPool`, source default child order and retained batch lifecycle,
  typed nearest result, and explicit capability errors.

The actual existing `SimulationDmoAssemblyPool` remains the only DMO membership
authority. The constructor retains that pool's unique `Arc<()>` occurrence;
query execution checks pointer identity as well as manager/backend identity.
This closes a review finding: manager/backend equality alone would allow a
second empty pool under the same manager to replace an admitted DMO owner.
ConstraintSession's outer bridge also passes its own retained DMO field.

The dynamic collection projection reserves backing capacity and exposes zero
published entries. It has no mutation or mesh-publication API. It does not
claim to reproduce unobservable per-capacity native Reset arithmetic on unused
backing slots. Actual empty collection iteration uses the existing
`try_publish_collection_line`; no callback simply returns a fabricated miss.

Any admitted DMO membership refuses **at DMO child execution**, after earlier
Ground End effects, because publication is not implemented. Failure leaves
the DMO batch Running/incomplete and later vehicle batch in AddTests. Acquired
leases and completed effects are retained; no automatic cleanup converts
failure into a successful miss. A distinct absent-owner error belongs to the
enclosing session bridge. Query results currently have null dynamic owners
because dynamic membership cannot execute until a publisher exists.

Ground remains an explicit borrow of the same retained pool used by the
authored-ground solver. This is a scoped actual component connection; it is
not a claim that the full native persistent World lifecycle has been rebuilt.

## Validation defined, results pending root checkpoint

Seven focused tests in `simulation_world_line_query_tests.rs` cover:

1. Actual empty constructor owners without source images, repeated query and
   explicit release/reacquisition of all three children.
2. Flag-1 omission of vehicle queries while retaining its capacity-24 owner,
   and flag-3 vehicle exhaustion preserving earlier acquisitions.
3. Unsupported selected flags and foreign-manager/backend/DMO refusal.
4. Ignored AddTest capacity refusal, later child execution and per-child
   aggregate count guards.
5. First tie/unordered retention, nearer replacement and full result copying.
6. Same-manager second empty DMO pool cannot replace the bound occurrence.
7. Real test-candidate construction and Insert into the actual bound pool
   forces DMO refusal, preserving Ground Running/complete, DMO
   Running/incomplete and vehicle AddTests states.

`simulation_world_line_corpus.rs::assert_original_world_line` accepts the
actual initialized session, original ground pool and the caller's original
probe input. It checks the existing independent simplified original
thin-segment oracle against the actual Ground child, then compares two full
World queries against that child including W lanes, provenance, owner and
hit count. Its caller is integrated by root into the original terrain corpus.
The independent oracle's numerical tolerances are the established ground
packet's, not a Xenon-bit-exact claim.

At author handoff only rustfmt was run on the new Rust files. Cargo checks,
original-data execution and independent review completion are root-owned
pending validation; no pass count is asserted here.

## Remaining boundaries

No `BeginUpdate` is added. Native `DynamicPool::BeginUpdate` `827c93f0`
synchronizes all four families, clears mesh count, traverses actual members,
and reaches base `AssemblyPool::BeginUpdate` `82e0dc98` even when empty.
The base empty parallel route schedules no jobs, but the empty nonparallel
route still populates job input and schedules AssemblyPoolUpdateJob at
`82e0df08`. An unconditional empty-update no-op would not be source parity.

DMO spawn/admission breadth, mesh publication, vehicles with members and
complete outer Island scheduling remain parked or unsupported. Active IK
aggregation, controller history/gestures, actual board forces, Actor/state
scheduling and portable presentation have their separate owners and remaining
gates. This packet completes no playable-controls or deterministic native
physics-parity claim.

Confidence: **high** for inspected native constructor/query ownership and
ordering; host factoring is implementation design. Original runtime trace
parity remains unverified.
