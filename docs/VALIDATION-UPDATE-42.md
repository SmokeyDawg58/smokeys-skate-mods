# Update 42 validation

The publicly distributed Windows `skate2-pc.exe` has SHA-256:

```text
16de0392bda3e1444f39f21f0e51f679a61f580bd7c29c2722d80d378ab8ba60
```

It was built from the clean integration commit `9ba85fd628345e4bbf9b306631a4df9eebf0172f`, based on upstream commit `5000b47437a7d9fc076613d9519ed9e82d5aabb6` (0.2.9). The public source snapshot externalizes the game-derived pool input and adds documentation; see [Building](BUILDING-SMOKEY.md).

The release record reports:

- A successful clean Windows release build and **88 audio unit tests**.
- Eleven real-data feature checks, including marker/bail, props, manual scoring, darkcatch animation, grabs, footplants, difficulty, and online map resets.
- A 740-tick actual-game off-board marker check, including a completed return and 8.9 metres of subsequent walking, with no watchdog warnings.
- Two sampled Skate 3 darkcatch events in an actual-game audio trace; an ordinary kickflip retained its native audio policy. Rendered diagnostic audio was finite and non-silent.
- Successful online sign-in and lobby browsing for the exact release EXE.
- Separate updater fixtures covering signed installs, backups, an official fallback, and restoring a compatible mod.

The externalized public source snapshot separately passed `cargo check --locked --offline -p skate2-pc` on Windows with the matching extracted build inputs. This is a compile check, not a new release build or a gameplay test; the release build and runtime checks above apply to the original integration source and the exact published EXE. On the test PC, SDL required Ninja and embedded C/C++ debug information to work around an installed Microsoft compiler debug-database mismatch.

The automated online check did **not** play a public multiplayer challenge. Original-data probes do not cover every career scene, challenge, map, hardware configuration, future upstream release, or visual issue. All automated game runs used scratch profiles and audio dumps rather than changing the player's volume settings.
