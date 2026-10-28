# Live selected frame services

This packet extends existing owners only. `PhysicalGameInfo` retains its actual
signed sync level. `SimulationAssemblyConstraintSession` retains the projected
manager Params field in an Arc shared by its feedback adapter. No new world,
backend, input or camera owner is introduced.

Fresh raw/decompile evidence from the reference IDB:

- `GameInfo82559418`: `li r11,0` at `82559440`, `stw r11,0x10(r31)` at
  `82559470`. The native static owner is `CommonBackEnd::sGameInfo83040500`.
  `DoCommand825e5610` first gates on `addToCRC`, then reads the actual signed
  level and includes command CRC only for level>=2. `CommonBackEnd` construction
  writes zero again at `825623f8`, but also changes game mode at `825623f0`.
  The selected static-field projection does not claim that complete constructor.
- `kEnableFreezing`: ref `83052764`, initializer `82f6a5d0`, constructor call
  `82f6a5fc`, table `82fd2248`; section `Physics_Simulation`, label
  `kEnableFreezing`, title `Enable Freezing`, true, null callback. This table row
  follows the selected ghost initializer at `82fd1d1c` and precedes individual
  full bind at `82fd2414`. API index292 follows the async BlendSpace288..291
  reservation; initializer order is separate from the manager's section/title
  ordering.
- `Step_Solver3`: RF reset at `827bae4c`, live reference target load at
  `827bae58`, target+0xcd byte load at `827bae5c`, zero branch at `827bae64`,
  freeze at `827bae6c`, contact spies at `827bae74`. There is no null target
  default. Existing cvar `try_current_boolean` preserves a typed unlinked error.
- World Initialize sets manager Params+0x24 to2 at `827b0f20` and `827b0f7c`
  for its network/local managers. Manager construction copies its Params at
  `82e0c364`; raw `82e0c718` reads supplied Params+0x24 and `82e0c71c` stores
  that value to the actual Simulation+0xa8 cooldown. The threshold remains
  separately retained: `FreezeIfNecessary82e0be58` samples manager+0xcc and
  compares unsigned body cooldown before its existing bounded capture/freeze.

The selected World feedback initializer implements those two distinct writes,
once, on the actual existing manager/pool. It does not replace pool Initialize's
independently correct initial30, nor claim other World constructor fields.
Adapter construction retains the actual Params allocation and live cvar table;
the enabled flag and threshold are sampled only by the existing consumer phases.
The caller remains responsible for supplying its same-world adapter.

The repeated original-data driver now initializes these Params before its
selected predictor/player construction, uses the same world GameInfo Arc for
command sync, and passes the live feedback adapter. Earlier isolated controlled
fixtures are unchanged. The neutral input, selected spawn, explicit scheduler
and parked-grind fixture policies remain explicit; no complete session is
claimed.

Focused tests check unlinked cvar refusal, native true default, later live
target changes, actual zero sync, retained threshold independent of changed
pool cooldown, and duplicate constructor refusal. The enabled feedback test
uses a stationary component fixture, two actual DynamicUpdate operations to
produce a qualifying cooldown, existing real completed constraint/export
receipts, then live default freezing. It verifies threshold2 freezes the body
while replacement cooldown73 is retained, and RF reset/completed progress.

Draft rustfmt and `git apply --check` pass. Independent review requested.
No author Cargo run; root owns integration, strict checks and mounted tests.

Root checkpoint: strict clippy-attrs-live-services-2 PASS41.17s; workspace-attrs-live-services-2 4,633 unit plus six doc PASS; original-attrs-live-services-1 18/20,5.01s. Neutral stops at real BlendSpace serialized normal tail; bail advances past Action23 to Motion expression268.
