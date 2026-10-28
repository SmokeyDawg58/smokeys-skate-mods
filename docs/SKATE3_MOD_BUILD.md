# Embedded Skate 3 player mod

This is ongoing PC mod work. See `evidence/pc_skate3_runtime_integration.md`
for tested behavior and remaining features. Do not advertise the menu entries
as completed simulation implementations.

The PC executable embeds the captured Skate 3 player's required animation,
graph, input, skeleton, collection, original gesture hand palettes and additional
trick-name language assets. At runtime it requires the
original Skate 2 game folder, just like the base port. No separate Skate 3
installation, extracted player pack or asset folder is used at runtime.

To rebuild, supply your local captured engine's asset directory as a build
input, plus the English language files extracted from the owned Skate 3 miscboot
archive. The localization directory contains `data/fe/languages/english/`.
Keep both inputs and the resulting pack outside this source repository:

```powershell
python tools/build_skate3_embedded_assets.py --assets C:\path\to\skate3-engine\assets --localization C:\build-inputs\skate3-language --output C:\build-inputs\skate3-player.pack
$env:SKATE3_EMBEDDED_PACK = 'C:\build-inputs\skate3-player.pack'
cargo build --release -p skate2-pc --locked
```

Alternatively, write the absolute pack path on the first line of the ignored
`skate3-pack-path.txt` and run `build-release.bat`. The pack must also exist
when rebuilding real-data tests with the `skate3-runtime` feature. The server
does not enable this feature.

Do not commit or publish original game assets, the generated pack, machine
configuration, saves, or release keys. Keep the captured engine's original
checkout unchanged; integration edits belong in `vendor/skate3-engine`.


Current playtest stage: full Skate 3 physics on the native Skate 2 character.
Only the master physics switch is exposed until that build is confirmed.
Skitching is skipped by request because upstream has no connected state;
the separate mixed-physics switches remain a later stage. Gesture catalogs,
separate saved D-pad choices and same-character playback are integrated; see
`evidence/skate3-gesture-actor-and-marker-regressions-2026-10-21.md` for validation
and editor-preview limitations.


## Authorized release-server network compatibility

The human reports creator Wispp granted permission to make this physics client
compatible with the public release server. The connection proof checks, transport
encryption, version check and server permissions are unchanged. A compatibility
build can set `SKATE2_NETWORK_BUILD_SECRET_FILE` to a private build input containing
exactly 32 bytes of an authorized release's network identity. The build script
embeds it into the executable; the completed game never loads that input file.
It is separate from the private release/update SIGNING key and cannot be combined
with `SKATE2_RELEASE_SECRET`. Without it, the existing release-key or random-build
paths retain their behavior. Do not put a network identity in the repository,
logs, or a standalone file in a distributed test package.

Test4 targets release 0.2.4, server 172.237.109.212:32082, public build ID3548cb37.
The authorized identity was verified against the official release's own loopback
server startup. Verification of actual public sign-in and the Lobby Browser is
recorded separately in the test's validation report; compilation alone is not
proof of an online game. No update signing key was read or used.
