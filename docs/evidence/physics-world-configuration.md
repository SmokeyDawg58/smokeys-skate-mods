# Actual World physics configuration and island-label boundary

2026-09-07 read-only live IDA/PDB and original-archive audit. Confidence is
`exact` for cited stores/layout/raw words, `high` for the interpreted route.
No new gameplay configuration or partition implementation is claimed here.

## Gravity is a game attribute, not the library default

Island constructor `0x827bab18`, after InitWorld/InitCullingTable, copies the
16-byte attribute layout vector directly into Simulation gravity `+0x90` at
`0x827bad30/34`. Island::UpdateWorldFromAttributes (`0x827b2cd8`) performs the
same unchanged copy at `0x827b2ce8/ec`. World::UpdateWorldFromAttributes
(`0x827ae3c8`) forwards to its islands. The writer does not rewrite already
accumulated body forces. Configure before adding gameplay bodies; later force
restoration is the existing DynamicUpdate behavior.

PDB `Attrib::Gen::physics::_LayoutStruct` is 0x20 bytes: WorldGravity at +0,
SkateboardStartPos at +0x10. Class `physics` key `04293d0f2adcfbb4`, default
collection `d7edbd362d7d2152`; WorldGravity key `48b8137e1b32a2e4`, size 16,
offset 0, count 1, flags 6, alignment 4. The actual default layout points to
skater.bin+628672. Original db.big BIN entry begins at 851840; direct bytes at
that entry plus layout offset are `[00000000,c11ccccd,00000000,00000000]`,
or `[0,-9.8,0,0]`. This is class physics, NOT physics_world. Raw archive identity
is recorded in [wheel evidence](skateboard-wheels.md).

Generated attribute constructor `0x827b2660` uses DefaultDataArea(0x20) only for
a null layout pointer. Live changed attributes propagate at the END of
World::Update3 through the watcher call `0x827afa7c`, not automatically at
StartFrame. Existing backend/pool set_gravity is the field-only reuse seam;
its XYZ representation must not be mistaken for proof of arbitrary native W
lane behavior. The shipped gravity W lane is zero.

Follow-up [Vault fetch audit](vault-runtime-fetch.md) establishes that this
generated accessor reads retained layout storage directly, not generic
GetAttributePointer(WorldGravity). The latter first searches explicit child
and parent nodes and can therefore differ from generated access. There is no
automatic default-collection retry. The watcher also subscribes to the generic
default collection key: a physics.default edit queues skateboard replacement
BEFORE the world-attribute update. Exact mutable edit ingress and the full
replacement path are not implemented; a field-only setter is not that service.

## Other gameplay overrides

World::Initialize `0x827b0e58` sets manager Params timestep `3c888889`, freezing
threshold `3727c5ac`, cooldown 2 and iterations 25. Manager constructor
`0x82e0c298` copies timestep (`0x82e0c2ec`), minimum energy (`0x82e0c70c`),
cooldown (`0x82e0c71c`) and max iterations (`0x82e0c6e4`). Library defaults
gravity -600, minimum energy 1 and cooldown 30 are NOT these game settings.

Island::Step_UpdatePrePhysics1 (`0x827b36e8`) overwrites the iteration limit
each tick (`0x827b3744`). FLAG_Both (3) selects the single-player cvar. Its
initializer `0x82f6a620` supplies default 50, minimum 25, maximum 200; cVarIntRef
constructor `0x82a8ae58` confirms argument meanings. Use its current value,
not constructor 25 or the corpus's controlled four iterations.

## Why an all-zero island map is not a complete offline frame

World::BeginDMOUpdate (`0x827adec0`) invokes the authoritative NetworkSimulation
StartFrame for simulation flag bit 1, even offline, then optional visual
simulation and prephysics. StartFrame (`0x82e0bd80`) clears relevant counters
and ten per-island count sets, walks ONLY Active-ring bodies in right-link
order assigning label zero, then assigns the dedicated static body minus one.
Frozen/other labels are retained; constructor initially fills all labels -1.

Later Island::Step_Solver2 (`0x827b35b8`) FLAG_Both invokes
SetupSimulationIslandsForSinglePlayer (`0x827b2880`): HookPool assemblies' Parts
are marked -1 in pool/Part order via SetAssemblyIsland (`0x82e0aff8`), followed
by drive normalization, optional thawing, assembly-pair partitioning
(`0x82e0e2d0`) and CreateSimulationIslandTables(...,0).

The newly integrated [persistent label owner](persistent-island-labels.md)
now produces StartFrame bytes and current-occurrence snapshots. Active0/static-1
is justified for an explicitly pair-free non-Hook restricted path, not as
replacement for actual offline partitioning. A separate
[partition source packet](assembly-island-partition-source.md) records the
recycled-label merge and required eligibility-pair producer, still unimplemented.
Runtime ghost-mode choice and original per-tick traces remain open.
