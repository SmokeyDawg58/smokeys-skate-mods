# Install and update

## Before you start

- Windows 10 or 11, 64-bit, with the graphics and CPU requirements of the upstream [Skate 2 Rust Port](https://github.com/skate6743/skate2-rust-port).
- Your own extracted Xbox 360 Skate 2 game folder containing `default.xex` and `data`.
- A working Skate 2 Rust Port installation and a controller. This mod's published package has been checked on Windows; Linux and macOS mod builds are not verified.
- Internet access for the installer, updates, and online play.

The installer does not contain your game files. The published mod EXE contains its prepared Skate 3 integration assets, so a separate Skate 3 installation is not required to play that binary. Source builds require local build inputs; see [Building](BUILDING-SMOKEY.md).

## Recommended: installer

1. Download `Smokeys-Skate-Mods-Installer-Windows.zip` from this repository's Releases.
2. Close Skate 2 and extract the ZIP.
3. Put **Smokey's Skate Mods Installer.exe** in the folder that contains your existing `skate2-pc.exe`. The `ROM` folder is game data; it is not the installer's destination.
4. Open the installer and wait for it to finish. It downloads the signed mod and keeps a backup of the replaced EXE when possible.
5. Open your usual game icon. If needed, enable **All Skate 3 Physics** under Options → Mods → Skate 3 Physics. Weather and Water Physics are also under Options → Mods, and rain volume is in Audio Settings.

The installer may open the game after a successful installation. If it reports that everything is already up to date, use your normal game icon. The game creates **Smokey's Skate Mods Updater.exe** beside the game; after it appears, you may remove the separate installer EXE.

## Direct EXE

The Update 42 Windows release also offers a ZIP containing the tested `skate2-pc.exe`. The installer is preferred because it records the installation and preserves a backup. If using the EXE directly, close the game, make your own backup of the existing EXE, and copy the new one beside it. Direct replacement alone may not register the installation with the updater.

## Future updates

Follow the game's **Install Update** prompt, or close the game and run Smokey's updater from the game folder. Keep using the same game icon afterward. Existing users can use their current updater to install Update 42.

If an official game update arrives before a matching mod, an established mod installation can switch to the verified official game through the updater. The mod is temporarily inactive, the updater remains available, and it checks again when launched later. The fallback requires an available, verifiable official Windows release. A compatible mod must be published before it can be restored.

## Uninstall

Close the game, open **Smokey's Skate Mods Updater.exe**, and choose **Uninstall mod**. The updater restores a verified pre-mod backup when one exists. If it does not have a verified backup, it asks you to select a known original executable; it does not invent one. Saves and settings are not intentionally removed by the uninstall operation.

For installation or online issues, see [Support](SUPPORT-SMOKEY.md).
