# Smokey's Skate Mods — Update 42

**Windows x64 · based on Skate 2 Rust Port 0.2.9**

Skate 3-inspired physics, darkcatches and catch audio, Clean/Sketchy landing notices, rainy weather, water effects, optional slippery-water physics, and the Smokey updater are included in this release. See `docs/FEATURES-SMOKEY.md` in the repository for the full list and limitations.

The updater can offer a verified official-game fallback for an existing mod installation when a newer official release has no matching mod yet. It checks for a compatible mod at a later launch. Future code conflicts may still need a hotfix.

## Downloads

- **Smokeys-Skate-Mods-Installer-Windows.zip** — recommended for new users. Extract it and put the installer beside your existing `skate2-pc.exe`.
- **Smokeys-Skate-Mods-Update-42-Windows.zip** — the tested game executable and licenses for manual installation. Back up your current EXE first; direct replacement alone may not register the installation with the updater.
- **SHA256SUMS.txt** — checksums for the release files.

You need your own extracted Skate 2 game files. Windows is the validated platform for this mod release. Source builds require local game-derived inputs; see `docs/BUILDING-SMOKEY.md` in the repository.

The game EXE SHA-256 is `16de0392bda3e1444f39f21f0e51f679a61f580bd7c29c2722d80d378ab8ba60`.

Update 42 passed a clean build, eleven feature checks, audio checks, an actual-game marker test, and online sign-in/lobby browsing. Automated testing did not include a public multiplayer challenge. See `docs/VALIDATION-UPDATE-42.md` in the repository.

**AI disclosure:** AI assisted throughout the project, including research, implementation, testing, and documentation. Smokey directed and tested the work alongside community contributors. See `CREDITS.md` in the repository for the upstream authors and third-party notices.
