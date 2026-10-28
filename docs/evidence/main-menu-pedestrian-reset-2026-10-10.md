# Return to main menu: stale pedestrian animation roster

Report: career quit after 31,072 completed frames failed in the front-end
world with `pedestrian PRES EndCommands: UnsupportedSetup`.

## Cause and reference

Confidence: high for the missing lifecycle step; the exact reported long-session
state has not been reproduced in a gameplay run.

- `game_unload` removed challenge-owned pedestrians and skaters but omitted the
  ambient living-world reset. `try_tick_frontend_world` disables living-world
  simulation while retaining the recipe manager. Its presentation pass therefore
  still visited the remaining career pedestrians.
- `TreeCommands::replay_pedestrian` preserves each recorded SIM stream's allocation
  range. `retain_sqt_program` requires consecutive streams to share an owner and
  adjoining allocation ranges. Removing only some members after their last SIM
  update can leave a discontinuous roster. The reported error is this
  precise validation, not a missing animation codec.
- IDA `StateGameUnload::UnloadGameState` **0x82757cc8**, call **0x82757e04**, and
  `GameModule::GameUnload` **0x8273d1b0**, call **0x8273d994**, both call `ResetGame`.
  `ResetGame` **0x8273a680** calls `SimController::Reset` **0x828a2c68**, which calls
  `Simulation::Reset` **0x8289eec8**. Its call at **0x8289effc** resets LivingWorld.
- `LivingWorld::Manager::Reset` **0x8271d3d8** updates spawn enable, resets ChaseMan
  and MoodMan, clears CensusTargetCache, unregisters plugins, requests all culls,
  and runs Census.Update. It then clears Big Black, rebuilds waypoint registrations
  from the retained chunks without messages, and resets the component entities.
- `ForceAllPluginsUnregistered` **0x826f4d18** and pedestrian specialization
  **0x82e83170** call `RemoveBehaviourPlugin` before any cull. Pedestrian removal
  **0x82e79ca8** clears waypoint/head-tracking references before base removal
  **0x82e4c3e8**. Existing port methods supply these operations.
- `RequestAllCull` **0x826f50d0** visits pedestrians, vehicles, dynamic objects,
  then props, with `bCensusManagedOnly=false`. Pedestrian specialization
  **0x82e993e0** uses each entity's explicit cull. `CensusMan::Update`
  **0x82702780** destroys detached pedestrians, vehicles, props, then dynamic
  objects. Existing cleanup handles physics bodies, road reservations, contained
  props, recipe/animation allocations, monitors and behaviour sources.
- Component Reset slots fold to **0x82558fe8**, calling ResetAllEntities. After
  this reset only rebuilt waypoint Props remain. Prop's Reset slot at
  `LWBasicEntity` vtable **0x82394b00 + 0x5c** points to no-op **0x825187c0**.

## Changes

- Added `selected_living_world_game_reset.rs` and called it from the shared
  `game_unload` path before the menu's first world tick. This covers career,
  profile free skate and Party Play. It uses the existing cull/destructor methods;
  it does not skip or relax animation validation.
- Exposed the existing dynamic-object cull bridge to the reset module.
- The host keeps its world and graph owners across games. Retire its actor-tracker
  ID when `ActorTracker::~ActorTracker` (**0x82e66de0**) frees the associated
  payload. Run existing `InitPropData` again when entering a game through
  `set_game_mode`: native `GameLoad` **0x8273c01c** calls `PreLoadStateGraphs`, whose
  call at **0x8272550c** runs `InitPropData` **0x8271d610**. This is host lifetime
  handling around the retained owner; it does not claim the native Reset function
  itself writes `mpActorTracker = nullptr`.

## Validation

- Live rich-symbol IDA read-only audit; exports in
  `.scratch/menu-unload-20261010/ida-reference.c`.
- `cargo check -p skate2-pc -j 1` passed before the final actor-tracker lifetime
  addition. First check found a helper visibility error, corrected by sharing the
  existing cull bridge; second check passed in 36.58 seconds.
- `cargo build --release -p skate2-pc -j 1 --timings` passed with all changes,
  including the actor-tracker lifetime addition, in 13m 16s. Existing warnings
  remain; the rebuilt executable is `target/release/skate2-pc.exe`.
- No tests added or run; no gameplay reproduction performed. Runtime coverage of
  quit while populated, repeated quit/re-entry, and active guards remains pending.
- Real `saves/SKATER.P` unchanged during this work, SHA256
  `C2B071DA246EF664ED1353D8A1244C330F45A03296EA27ABC4F7D5C73118D297`.
  Scratch backups, build logs and IDA exports are under
  `.scratch/menu-unload-20261010`.
