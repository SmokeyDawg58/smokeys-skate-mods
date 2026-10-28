# Support

For a bug, open an issue in this repository and include:

- The Smokey update number and official Skate 2 Rust Port version.
- What you expected, what happened, and steps to reproduce it.
- The mode and map: career, free skate, online lobby, or a named challenge.
- Relevant options, such as Skate 3 Physics, difficulty, weather, and Water Physics.
- The exact error message and whether the stock upstream game has the same issue.

If it is safe to share, include a screenshot or a short log excerpt. **Review logs before posting:** they may contain your account nickname, local paths, input, network information, or other details. Never post passwords, GitHub or hosting tokens, game files, original ROMs, personal saves, or private keys. Report an exposed token to its provider and revoke it.

## Where are the logs?

Normal game runs write support data under **Mod Logs** beside `skate2-pc.exe`. Individual `run-*` folders can contain a build fingerprint, an executable hash, rolling event files, and failure reports. Close the game before copying a log folder. An abrupt shutdown can lose the last buffered records, and a stall report is not automatically proof of a crash.

Some older diagnostic builds use `saves/test_logs/` or a developer-selected `SKATE2_TEST_LOG_ROOT`; see [Testing logs](TEST_LOGGING.md). Keep the original folder intact and share only what you choose after review.

## Update or online mismatch

If the game says it doesn't match the server, choose the game's **Install Update** option or close the game and run Smokey's updater. If the mod has not caught up with the official game, an existing installation may offer the official-game fallback so you can play without the mod while a compatible update is prepared.

The updater checks at launch; it does not promise that every upstream update can be merged automatically. A missing upstream source release, changed game assets, verification failure, or incompatible code may require a repair. If the official download cannot be verified, the updater keeps the existing installation rather than silently installing an unknown executable.

## Audio and performance

Rain volume is separate under Audio Settings. Heavy rain and water can cost more performance than light rain; try a lower weather setting, graphics settings, or a lower frame-rate target. Do not reset system volume as a troubleshooting step.

For uninstalling or restoring a backup, see [Install and update](INSTALL-SMOKEY.md).
