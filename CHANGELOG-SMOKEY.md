# Smokey's Skate Mods changelog

## Update 43

## New and improved

- **Debug camera:** Fly around in free roam with Right Stick Click + D-pad Right. Press B to move your skater to the camera, or use the shortcut again to return without moving. Change the shortcut under Mods > Keybinds.
- **Camera controls:** Use the sticks to move and look, D-pad Up/Down to change height, D-pad Left/Right to adjust the field of view, and RT for smooth speed control up to 20×.
- **Object Dropper and park pieces:** Build a spot with props, characters, mission pieces, terrain, and Skate 3 park pieces, with snapping and rotation controls. Open it with LB + D-pad Left; Contacts stays on LB + Y. The dropper is available in career and free skate, including online free skate, and closes when a challenge starts. Placed objects are local and are not synced to other players.
- **Menus:** Open Mods directly from the pause menu, above Quit Game. Weather and water physics have their own settings, and the Edit Style menu shows when there are more options below.
- **Weather:** Rain, clouds, lightning, puddles and optional slippery water physics are included, with separate rain strength settings and an after-rain setting. Offline pause stops the weather; online play keeps it moving through menus.
- **Skate 3 features:** Skate 3-inspired physics and difficulty choices, darkcatches, footplants, catch sounds and landing feedback remain available alongside the regular Skate 2 mode.
- **Updater:** The updater checks for releases and can fall back to the official game when a compatible mod build is unavailable, while keeping the updater for a later fix.

The debug camera is a free-roam tool; online camera behavior still needs validation. The experimental Skate 1 physics are not part of this update. You need your own Skate 2 game files; the prepared mod build includes the park pieces, so no separate Skate 3 install is needed.

---

## Update 42 — Skate 2 Rust Port 0.2.9

- Incorporates the current shared Skate 3-inspired skating, scoring, weather, water, and menu features into the 0.2.9 integration.
- Uses the upstream controller settings rather than the previous separate Smokey controller menu.
- Keeps the game update notice and Install Update handoff.
- Adds a verified official-game fallback for an existing mod installation when the published mod does not match the latest official game. The updater remains available and checks for a compatible mod on later launches.
- Preserves the updater's signed-feed verification, backups, and uninstall flow.

Full checks and limits are documented in [Update 42 validation](docs/VALIDATION-UPDATE-42.md). Earlier feature history can be found in the source evidence documents; this changelog does not reconstruct a complete release-by-release history.
