# Rebuilding the Test 12 mod

This source contains the supplied Test 10 integration, the Test 11 marker
correction and online diagnostic, and the Test 12 darkcatch audio adaptation.
The source evidence documents the observations, policy and validation limits.
The upstream game version remains 0.2.5; Test 12 is the mod's package label.

The verified Windows build used Rust 1.98.1 with the MSVC target and the
workspace's locked dependencies. The game data and embedded player pack remain
external build inputs, as required by this repository's contributor guide.
The supplied playable EXE already embeds its player pack; playing requires no
separate Skate 3 installation.

For a release build, set these environment variables to your local inputs:

* `SKATE3_EMBEDDED_PACK`: the existing hash-verified SK3PACK1 player pack.
* `SKATE2_NETWORK_BUILD_SECRET_FILE`: optionally, an authorized 32-byte network
  compatibility input for this specific release. It is not an update signing
  key. Without one, normal development-build identity generation applies.
* `SKATE2_DEFAULT_SERVER`: the intended server address and port.

Build from the workspace root with `cargo build --release -p skate2-pc --locked`.
No release signing key was supplied or generated for this custom build.
Private build-input sidecars and compiler output are excluded from the shared
source archive.

Real-data checks use a separate profile copy through `SKATE2_SAVE_ROOT` and
the player's extracted Skate 2 game folder through `SKATE2_GAME_ROOT`. The
bounded `--live-controls-demo darkcatch-audio` checks actual darkcatch contacts.
Use a separate fresh scene with `--live-ollie-demo --live-ollie-pattern Kickflip`
to check ordinary flips, since a darkcatch bail may end the first scene offboard.
`--check-online` verifies this exact executable's sign-in and lobby browser.

The friends bundle includes this source archive and the Skate 3 engine's
license alongside the playable EXE. It does not include the player's personal
profile, nickname, game folder path or unpacked game data. The standard game
updater remains unconfigured: official replacement EXEs do not contain this
custom integration.
