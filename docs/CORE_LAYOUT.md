# Navigating skate2-core

The September 29 organization moved 1,977 files out of the flat `src` directory.
All 1,978 existing Rust files were retained, including untracked and currently
unreferenced files. This was a physical relocation, not a gameplay rewrite or
change to the crate's public API.

The directory overview is in [src/README.md](../crates/skate2-core/src/README.md).
The tab-separated [old-to-new index](core-file-moves.tsv) covers every moved file.
Historical patch files and one-time generation scripts remain historical evidence;
do not rerun them as current migrations. Use the index to resolve their old paths.

## Frequent starting points

| Task | Directory |
| --- | --- |
| Missing Action graph expression/state | `animation/graphs/action/` |
| Missing Motion graph expression/state | `animation/graphs/motion/` |
| Motion callbacks acting on the Actor | `animation/graphs/motion/callbacks/` |
| Animation pose, blend or timing problem | `animation/pose/`, `trees/`, `channels/`, `runtime/` |
| Ground, air, offboard or grind physics | `physics/player/ground/`, `air/`, `offboard/`, `grind/` |
| Handplants, footplants or bails | `physics/player/handplant/`, `footplant/`, `wipeout/` |
| Physics output and conditioning | `physics/player/output/` and `conditioning/` |
| Selected physics state/frame wiring | `physics/player/lifecycle/` and `runtime/` |
| Board pushes, braking or steering | `physics/board/forces/`, `control/` and `physics/player/board/` |
| Skeleton IK or processed inputs | `physics/skeleton/ik/`, `processed/` |
| Rigid bodies, contacts, joints or drives | `physics/solver/` |
| Collision queries or assembly publication | `physics/collision/`, `physics/assembly/` |
| Cars or pedestrian navigation groundwork | `living_world/vehicles/`, `navigation/`, `population/` |
| Live LivingWorld integration | `session/living_world/` |
| Startup, selected session or chunk residency | `session/`, `session/startup/`, `session/streaming/`, `streaming/` |
| Controller devices, recognizers or cvars | `input/devices/`, `recognition/`, `tuning/` |

Paths in this table are relative to `crates/skate2-core/src`.

## Module and ownership rules

- Keep existing logical Rust module names and `crate::...` imports. `lib.rs`
  maps top-level modules to their files with `#[path = "subsystem/file.rs"]`.
  Existing child module owners similarly use relative paths. Visibility,
  `super::` relationships, cfg attributes, macro ordering and initialization
  semantics are unchanged.
- Put a new file in its owning subsystem folder. Keep tests/corpus helpers in
  that same area. Prefer extending an existing implementation over creating a
  second implementation with a different name.
- Before adding a file, search the whole crate recursively. Do not assume an
  absent file in the old flat root means the implementation is missing.
- Do not add forwarding copies at old locations, create empty folder modules,
  or duplicate module registrations. A filesystem folder need not be a module.
- Keep platform/window/GPU code in the app. These folders organize the existing
  core responsibilities; they do not move platform behavior into simulation.
- Use the full current path in new documentation. Existing basenames remain
  searchable, and older full-path references can be resolved through the index.

Examples from the repository root:

```powershell
rg --files crates/skate2-core/src -g '*handplant*.rs'
rg -n 'PendingExpression' crates/skate2-core/src/animation/graphs
python tools/graph_query.py motion state 20
python tools/check_core_layout.py
cargo check --workspace --offline -j 1
```

The layout checker verifies literal module/include targets and prevents a return
to a flat source root. It does not replace the Rust compiler or runtime testing.
