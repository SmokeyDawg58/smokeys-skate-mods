# Project layout

This repository is a source snapshot of the mod integrated with the upstream Skate 2 Rust port. The original upstream README is preserved as [UPSTREAM-README.md](../UPSTREAM-README.md); its download links and feature statements describe upstream, not necessarily the Smokey release.

| Path | Purpose |
| --- | --- |
| `apps/skate2-pc/` | PC game, renderer, weather, audio integration, game UI and launcher handoff. |
| `crates/skate2-core/` | Skate 2 simulation and integrated Skate 3 gameplay, options, challenges, and sessions. |
| `crates/skate2-audio/` | Audio, including the isolated Skate 3 catch bank. |
| `crates/skate2-formats/`, `crates/skate2-xma/` | Game-file parsers and audio decoding. |
| `crates/skate2-net/`, `apps/skate2-server/` | Upstream networking and dedicated server. |
| `vendor/skate3-engine/` | Vendored Skate 3 Rust Engine source and its GPLv3 license. |
| `tools/smokey-updater/` | Smokey's Windows updater, signed-feed checks, backup and fallback code. |
| `docs/evidence/` | Implementation notes, observations, test context, and limitations. |
| `tools/extract_update42_build_inputs.py` | Extracts matching private build inputs from the checked Update 42 EXE. |

`UPSTREAM-SYNC.json` describes an earlier 0.2.5 sync and the captured Skate 3 engine ancestry. For this published snapshot's 0.2.9 base and Update 42 validation, use [Building](BUILDING-SMOKEY.md) and [Validation](VALIDATION-UPDATE-42.md).

When updating against upstream, keep Wispp's game behavior authoritative and preserve the mod features. Prefer small integrations and isolated modules. Do not silently drop Skate 3 features to force a merge: stop and repair conflicts, build, and validate before publishing. Never put credentials, game assets, user profiles, logs, or local build folders into Git.
