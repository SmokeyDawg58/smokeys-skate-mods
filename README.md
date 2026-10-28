<div align="center">
  <img src="tools/smokey-updater/smokey-clean.png" alt="Smokey's Skate Mods logo" width="140">

  # Smokey's Skate Mods

  **Skate 3-inspired physics, weather, and more for the Skate 2 Rust port.**

  Windows · Update 42 · Based on Skate 2 Rust Port 0.2.9

  [Install](docs/INSTALL-SMOKEY.md) · [Features](docs/FEATURES-SMOKEY.md) · [Support](docs/SUPPORT-SMOKEY.md) · [Build](docs/BUILDING-SMOKEY.md) · [Credits](CREDITS.md)
</div>

Smokey's Skate Mods is a community-made modification of Wispp's [Skate 2 Rust Port](https://github.com/skate6743/skate2-rust-port). It brings parts of the Skate 3 skating experience into Skate 2, adds weather and water effects, and keeps the settings in the game.

> **AI disclosure:** AI was involved throughout this project, including research, implementation, testing, and documentation. Smokey directed the project, selected features, and tested builds with the community. The upstream authors' work and community contributions remain essential. See [Credits](CREDITS.md).

## What it adds

- **Skate 3-inspired skating:** physics options, difficulty presets, darkcatches and their sound, footplants, off-board movement, and scoring changes.
- **Weather:** light, medium, and heavy rain, snow options, and a "30 min after rain" look, with storm clouds, lightning, thunder, and adjustable rain audio.
- **Water:** puddles and water in supported pools, rain impacts, and optional slippery-water physics with deep-water falls.
- **Presentation:** Clean and Sketchy landing notices, mod information, and update notices in the game menus.
- **Recovery:** a signed shared updater, an uninstall option, and an official-game fallback when an installed mod no longer matches the latest game.

Read the [feature guide](docs/FEATURES-SMOKEY.md) for details and limitations. This is a work in progress, not a claim of exact Skate 3 parity.

## Play

You need Windows 10 or 11 (64-bit), a compatible controller, a working Skate 2 Rust Port installation, and your own extracted Xbox 360 Skate 2 game files. The extracted game folder contains `default.xex` and `data`. An internet connection is needed to download updates and play online.

1. Open this repository's **Releases** and download `Smokeys-Skate-Mods-Installer-Windows.zip`.
2. Close the game, extract the ZIP, and put the installer beside your existing `skate2-pc.exe`—not inside `ROM`.
3. Run the installer. It downloads the checked mod release and installs it at the usual game EXE path.
4. Open the game using your usual icon. Find gameplay, weather, water, and audio choices under **Options**.

Already have Smokey's updater? You can use that instead. See the [installation guide](docs/INSTALL-SMOKEY.md) for direct downloads, updates, and uninstalling.

## Updates and support

The updater checks the signed shared feed when opened or when the game asks it to check. Friends do not need Rust, Git, a GitHub account, or a compiler. It does not install a background monitor.

If a new official game version does not yet have a compatible mod, an existing mod installation can use the updater's verified official-game fallback. It keeps the updater and checks again at a later launch. A code conflict can still require a hotfix; future compatibility is not guaranteed.

For problems, open an issue in this repository after checking the [support guide](docs/SUPPORT-SMOKEY.md). Please do not post game files, saves, passwords, tokens, or unreviewed logs publicly.

## For developers

The full source is here. Start with [Building Smokey's mod](docs/BUILDING-SMOKEY.md), [Project layout](docs/PROJECT-LAYOUT-SMOKEY.md), and [Contributing](CONTRIBUTING.md). Original game data and private build/signing inputs stay outside this repository.

Update 42 was validated with the game build, eleven feature checks, audio checks, a marker-return run, and online sign-in/lobby browsing. A public multiplayer challenge was not part of that automated validation. See [validation details](docs/VALIDATION-UPDATE-42.md).

## License and attribution

The combined project includes GPL-3.0-only Skate 3 engine code and MIT-licensed upstream code. The original licenses and third-party notices are retained; see [Licenses](LICENSES.md). This unofficial project is not affiliated with EA or the upstream projects.
