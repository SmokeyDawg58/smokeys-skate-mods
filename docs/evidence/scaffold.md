# Scaffold and application loop evidence

Confidence: **high** for the call order and load-state transitions; **exact** for
the listed type layouts, enum values, timer values, and named direct calls.

## Reference symbols

| Address | Symbol | Object |
|---:|---|---|
| `0x8298e988` | `main` | `scaffold:scaffold_lib.cpp.obj` |
| `0x8298e810` | `systemInit()` | `scaffold:scaffold_lib.cpp.obj` |
| `0x8298d458` | `AppMan::Run(ExecutionMan*, int, char**)` | `scaffold:scaffold_lib.cpp.obj` |
| `0x82989ae0` | `AppMan::MainLoop()` | `scaffold:scaffold_lib.cpp.obj` |
| `0x82740fb0` | `Sk8::Sk8AppMan::Init(int, char**)` | game code |
| `0x82740458` | `Sk8::Sk8AppMan::InitBoot(int, char**)` | game code |
| `0x827349c8` | `Sk8::Sk8AppMan::InitApp()` | game code |

`mainCRTStartup` (`0x82c53310`) invokes the game-owned `main`. `main` calls
`systemInit`, constructs an Xbox execution manager, and calls `AppMan::Run`.
The native reconstruction replaces that platform manager while retaining the
observable application contract.

## Timing

`AppMan::Run` writes `mTimerHz = 60` and `mTimerPeriod = 0.016666668f` before
initialization. The corresponding Rust constants and tests are in
`skate2_core::scaffold`.

## Load-state machine

IDA type ordinal 2986 gives the exact `AppMan::eLoadState` values:

| Value | Original | Rust |
|---:|---|---|
| 0 | `kLoadIdle` | `LoadState::Idle` |
| 1 | `kLoadModuleActive` | `LoadState::ModuleActive` |
| 2 | `kLoadModuleFinished` | `LoadState::ModuleFinished` |
| 3 | `kLoadBackgroundActive` | `LoadState::BackgroundActive` |
| 4 | `kLoadBackgroundFinished` | `LoadState::BackgroundFinished` |
| 5 | `kLoadEnd` | `LoadState::End` |

The implementation preserves the original priority: a queued module load starts
before a queued generic background load. Completion callbacks run on the main
thread after the worker reports done. An active module can defer replacement or
shutdown by returning false from `LoseFocus`.

## Boot order recovered so far

`InitBoot` establishes filesystem/error handling, optional file-server sync,
events and message managers, the boot BIG group, asset registries, location
lookup, attribute vaults, jobs, language, graphics, input, and supporting debug
services. It loads these attribute vaults directly:

- `data\\db/skater.vlt`
- `data\\db/ChallengeCore.vlt`
- `data\\db/progressioncore.vlt`

`InitApp` then creates flow/audio/data systems, builds the trick database from
the attribute system, readies `DataSysMan`, and initializes renderer, character
assets, localization, front end, living-world recipes, textures, paths, scene
managers, loading, audio singletons, and networking.

