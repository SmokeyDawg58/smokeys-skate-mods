# Features and current limits

These features describe Update 42. They are community modifications, not claims of complete Skate 3 behavior or exact scoring parity.

## Skate 3-inspired skating

- **All Skate 3 Physics** as the in-game master switch, including darkcatch, footplant, and off-board behavior.
- Skate 3-inspired difficulty choices: Easy, Normal, Hardcore, Motorized, and Easy Motorized, alongside the Skate 2 setting.
- Darkcatch animation and the authored catch sound; normal kickflips retain their normal audio policy.
- Adapted air tricks, grabs, manuals, footplants, push behavior, scoring, and Hall of Meat/bail behavior.
- Clean and Sketchy landing notices near the multiplier.
- Marker, off-board, object/prop, and online-reset integration. Some modes, challenge combinations, and unusual maps may still behave differently.

Gameplay settings are local preferences. Enabling a modded physics option changes how your character plays; it does not promise an exact match to every original-game move, score, or challenge.

## Weather and water

Options → Mods offers Clear, Light Rain, Medium Rain, Heavy Rain, Light Snow, Heavy Snow, and **30 min after rain**. Rain volume is controlled separately in Audio Settings.

Rain includes drifting storm clouds, lightning, thunder, surface wetness, puddles, ripples, and water in supported pool/basin geometry. The after-rain setting has no falling rain and includes a faint rainbow and subtle bird ambience. The water is an authored visual model, not a full fluid simulation; unusual geometry can still look imperfect.

**Water Physics (Skate 3)** is optional and off by default. When enabled, wet wheel contact reduces grip, and sustained fast travel through deeper water can cause a fall. Effects depend on the locally sampled water surface and contact state.

Frame rate depends on hardware, map, weather intensity, and graphics settings. A 120 FPS cap is available in the upstream game; 120 FPS is not guaranteed in every scene.

## Menus and updates

The main menu includes mod information and update status. The native game offers controller-stick tuning; that control menu belongs to the upstream port rather than a separate Smokey controller mod.

The updater verifies signed mod downloads and can offer a verified official-game fallback for an existing installation if the current mod is incompatible with a newer official version. It checks on launch, not continuously. More details and limits are in [Install and update](INSTALL-SMOKEY.md).

## Online and testing

Update 42 was checked against Skate 2 Rust Port 0.2.9. The release validation covered online sign-in and lobby browsing, not a full public multiplayer challenge. Server changes, future game releases, specific challenges, and third-party mods can cause issues. Please report reproducible problems through the repository's Issues tab using the [support guide](SUPPORT-SMOKEY.md).
