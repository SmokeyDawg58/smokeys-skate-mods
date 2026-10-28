# Smokey's Skate Mods changelog

## Update 42 — Skate 2 Rust Port 0.2.9

- Incorporates the current shared Skate 3-inspired skating, scoring, weather, water, and menu features into the 0.2.9 integration.
- Uses the upstream controller settings rather than the previous separate Smokey controller menu.
- Keeps the game update notice and Install Update handoff.
- Adds a verified official-game fallback for an existing mod installation when the published mod does not match the latest official game. The updater remains available and checks for a compatible mod on later launches.
- Preserves the updater's signed-feed verification, backups, and uninstall flow.

Full checks and limits are documented in [Update 42 validation](docs/VALIDATION-UPDATE-42.md). Earlier feature history can be found in the source evidence documents; this changelog does not reconstruct a complete release-by-release history.
