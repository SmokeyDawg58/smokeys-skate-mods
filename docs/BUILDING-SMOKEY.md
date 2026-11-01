# Building Smokey's mod

For the debug-camera and park-editor update, the pool grid and player inputs are still external build inputs described below. Set `SKATE3_PARK_ASSETS` to your own Skate 3 `data/content/parkassets.big` when building the complete mod; the prepared distribution embeds it. A source build without this input does not include those pieces. Keep all game-derived inputs out of the repository.

The Update 42 extractor instructions below remain useful for obtaining the existing pool-grid and player inputs from that specific release; they do not extract the newer park archive.

This is the public source snapshot for **Update 42**, based on the Skate 2 Rust Port **0.2.9** source at `5000b47437a7d9fc076613d9519ed9e82d5aabb6`. The tested private integration was `9ba85fd628345e4bbf9b306631a4df9eebf0172f`.

The public snapshot changes the pool-ground input from a checked-in game-derived binary to an external build input, and adds project documentation. With the same input bytes the pool code receives the same grid; no physics or renderer algorithm was intentionally changed for this source publication. The published EXE was validated against the integration snapshot, not built again from this documentation commit.

## Prerequisites

- Windows x64, the Microsoft C++ Build Tools/MSVC toolchain, Python 3.9+, and Rust/Cargo. The workspace declares Rust 1.85 or newer; the published build used a later Rust toolchain.
- The matching Update 42 `skate2-pc.exe` from the release, plus your own extracted Skate 2 game files when running the game.
- A private folder **outside this repository** for game-derived build inputs. Do not commit those inputs, a ROM, a save, a release key, or a network identity.

The Update 42 executable SHA-256 is:

```text
16de0392bda3e1444f39f21f0e51f679a61f580bd7c29c2722d80d378ab8ba60
```

## Prepare local inputs

The helper checks the whole Update 42 EXE hash and each extracted section. It supports that exact published EXE only and refuses to overwrite different files. It recovers the player pack, catch bank, landing notifier, and seven-pool height grid into a private folder:

```powershell
python tools/extract_update42_build_inputs.py "C:\Games\Skate2\skate2-pc.exe" "C:\SkateBuildInputs"
```

The contents originate from game-derived data embedded in the matching published build. Keep them private and use game assets only where you have the necessary rights. The repository does not include the extracted pool grid, player pack, catch bank, or landing notifier.

For the complete mod build, point the build at those files:

```powershell
$env:SKATE3_EMBEDDED_PACK = 'C:\SkateBuildInputs\skate3-player.pack'
$env:SKATE3_DARKCATCH_BANK = 'C:\SkateBuildInputs\skate3-darkcatch.bnk'
$env:SKATE2_POOL_GROUND = 'C:\SkateBuildInputs\pool-ground.bin'
cargo build --release -p skate2-pc --locked
```

The landing notifier must remain beside `skate3-player.pack`; the app build script locates it there. If the Skate 3 pack is enabled without the pool-grid input, the build fails with a helpful message instead of silently omitting the supported-pool correction. A build without the Skate 3 pack can compile without the local pool grid; it is not the full Update 42 mod.

The rain beds and their credits are in the source. The old build script also accepts optional rain header/stream variables, but the current weather-audio path uses the included credited PCM beds.

### Windows compiler debug-database error

If the SDL native build fails with MSVC `C1902: Program database manager mismatch`, repair or update the Visual Studio C++ Build Tools first. A workaround used for the Update 42 source check is to use Ninja from a Visual Studio developer shell and place the following CMake toolchain file **outside** the repository (for example, `C:\SkateBuildInputs\embedded-debug.cmake`):

```cmake
set(CMAKE_C_FLAGS_DEBUG "/Ob0 /Od /RTC1 /Z7" CACHE STRING "" FORCE)
set(CMAKE_CXX_FLAGS_DEBUG "/Ob0 /Od /RTC1 /Z7" CACHE STRING "" FORCE)
set(CMAKE_MSVC_DEBUG_INFORMATION_FORMAT Embedded CACHE STRING "" FORCE)
set(CMAKE_POLICY_DEFAULT_CMP0141 NEW)
```

With Ninja available on `PATH` and the build-input environment variables above set, run:

```powershell
$env:CMAKE_GENERATOR = 'Ninja'
$env:CMAKE_TOOLCHAIN_FILE = 'C:/SkateBuildInputs/embedded-debug.cmake'
$env:CARGO_TARGET_DIR = 'C:/SkateBuildInputs/cargo-check-target'
cargo check --locked -p skate2-pc
```

Use a fresh target directory when changing CMake generators or toolchains. This checks compilation; it does not prove that an executable works online. These environment settings apply to the current shell.

## Online build identity

A local development build does **not** automatically share the public release server's build identity. Joining that server requires a matching, authorized input for the exact upstream version; no private network identity or release-signing key is in this repository. Do not publish or commit those values. Use the checked release for public online play, or follow the upstream project's authorized development/server workflow.

Updating the upstream game, source APIs, pack format, asset layout, or release identity may require code and build-input changes. A clean merge or successful compilation alone does not prove compatibility.

## Tests and diagnostics

Start with `cargo test -p skate2-audio --lib --locked` and focused tests appropriate to your change. The upstream contributor guide lists its broader checks. Game-data tests require your own `SKATE2_GAME_ROOT` and a scratch copy of saves through `SKATE2_SAVE_ROOT`.

For any automated game diagnostic, set `SKATE2_AUDIO_DUMP` to a **new per-run file** so audio is rendered without opening the speaker device. Do not modify normal game or operating-system volume settings. Public online checks use a dedicated diagnostic profile; do not upload saves or private logs.

See [Update 42 validation](VALIDATION-UPDATE-42.md) for what was actually checked and what was not.
