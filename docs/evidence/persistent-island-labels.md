# Persistent body-island labels

2026-09-07. `simulation_island_state.rs`, private child of the shared island
map, now owns the manager's signed-byte slot labels. Confidence is `high` for
the typed source reconstruction; addresses/layout/store order are exact source
facts. This is the label slice of StartFrame, not complete offline partitioning.

## Source and lifetime

Manager constructor `0x82e0c668..0x82e0c6cc` allocates m_RF_Max+8 bytes, sets
the byte pointer to allocation+8 and initializes ALL body-slot labels to -1.
PDB RigidBodySimulationIslandMap has size 8, Simulation pointer +0 and label
pointer +4. The map is installed before manager static Add at `0x82e0c744`.
The existing manager Contact owner now creates and retains this state before
its actual dedicated static-body allocation. A pristine-pool guard prevents
silently resetting the label map during a session.

StartFrame (`0x82e0bd80`) traverses Active bodies by right links, writes zero
to each slot byte (`0x82e0be0c`), then writes -1 to the dedicated static body
(`0x82e0be38`). Other static, frozen and free bytes are untouched. Empty Active
still performs the dedicated static write. The manager method always supplies
its retained static occurrence. Pool/frame/static validation precedes writes
as explicit safe-host preflight, not a native invalid-memory branch.

AddRigidBody `0x82ae6e00`, RigidBody::Reset `0x827c8f88`, and RemoveRigidBody
`0x82ae3190` do not update the label map. Therefore the bytes belong to native
SLOTS and survive release/reuse. Host snapshots bind each CURRENT non-Free
allocation occurrence to its retained byte; they exclude free slots without
clearing them. Old snapshots keep old occurrence identities and must be rejected
by downstream occurrence checks. Signed labels are not clamped or sanitized.

The shared map accessor remains slot-indexed and returns the stored observation
alongside its byte. Callers must compare that observation with the requested
one; lookup alone is not occurrence validation. The existing compiler checks
remain responsible for this comparison.

The owner rejects snapshot publication before a successful label refresh or
after the body frame advances. Initial body epoch 1 does not itself prove a
StartFrame. Same-frame changes still require correct scheduler chronology.
This operation does not reset all native Contact/Joint/Drive/pair/thaw counters,
advance the timestep, exclude HookPool assemblies, or partition collision pairs.

## Validation

Independent live-source review accepted constructor/store order, selective
reset, reuse behavior, immutable pool traversal and current-occurrence snapshot
publication. Eight synthetic tests pass: pristine initialization; no early
snapshot; active/static ordering and signed non-Active retention; empty Active;
remove/free/reuse with inherited byte and distinct occurrences; foreign/stale
static failure without partial writes; frame freshness; and Active traversal
order after reuse. Seeded prior labels in tests are explicitly synthetic and
do not constitute a partitioning implementation.

The real DMO/Rob's Park eight-frame corpus now calls the manager's label reset
and snapshot methods instead of constructing caller-supplied island pairs.
The controlled isolated non-Hook/non-pair path still yields moving body 0 and
dedicated static -1. Original first-contact fingerprint and eight finite
contact/integration steps remain unchanged. This validates pre-partition label
production for that path, not full offline island grouping or native tick parity.

Root validation after resuming the question break: all eight focused tests,
2223 workspace unit tests, six doctests, formatting and warnings-denied
all-targets Clippy pass. Both core ignored real-data tests were explicitly
rerun together and pass; the existing ignored app corpus was not rerun here.
