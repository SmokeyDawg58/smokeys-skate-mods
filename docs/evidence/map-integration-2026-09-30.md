# StartHub Map travel integration

Scope: open the original Map option and select destinations to teleport
around the installed city. Three parallel agents separated native menu and
map drawing, original APT translation, and travel/streaming integration. Root
integrated the shared asset/clip renderer and reviewed the boundaries. Existing
marker teleport, input, pause, localization, fonts, collision residency and city
GPU loader are reused rather than replaced.

## Source and asset evidence

- StartHub `OnMenuSelect82674d08` pushes frontend state21 for Map.
- ChallengeMap template14 at `82fdb438` is flat type0, not FocusLeft.
- `LoadMap8264ccf8` reads fe_locations default `d7edbd362d7d2152`, MapInfo
  `20bbc594cd2faef8`. The shipped record selects map512 with bounds
  X[-327,1858], Z[-1852,1539] and metadata dimensions329x512.
- `FEMapManager::RenderMap82631f70` draws the original texture with full default
  UVs over its custom APT shape. `MapIconManager::RenderIconOnScreenMap8262e350`
  adds original world-to-map offsets to its screen center. Icons use the shared
  original minimap arena, group icon references,32px size and64px selection.
- Native icon filtering follows highlighted category membership. Local skater
  uses its separately registered icon. No invented cursor texture is drawn.
- `GetCategoryHAL8261cb48` localizes and formats category item counts.
- `DoTeleport8265f280` -> `TeleportToFELocation82565350` uses the destination's
  authored location reference and existing offboard teleport flow, then exits
  frontend states. No synthetic coordinates or direct physics transform writes.

Original map, slider, LetterBox and dimmer occupy shared movie slots9..12.
Imported controls retain their parent screen projection. The dimmer is original
textured artwork with its authored alpha timeline, not an approximated overlay.
See [native evidence](map-native-2026-09-30.md),
[movie evidence](map-movie-2026-09-30.md), and
[travel evidence](map-travel-runtime-2026-09-30.md).

## Travel safety and scope

Catalog uses existing shared Vault and BAM location parser. The original four
location groups contain37 references; all37 resolve to installed main-world
transforms. This is the freeroam location browser, not fabricated career unlocks,
challenge ownership, online events, DLC travel or property purchase.

During travel, frontend pause remains held while destination collision
registrations complete and the existing asynchronous presentation loader/GPU
replacement finishes. The session then queues the existing Flow/PlayerUI
teleport and retains target streaming focus through its physical completion.
Once a travel load starts, navigation is held until it finishes. No second
terrain/presentation streamer was introduced.

The browser supports categories, destination selection, Teleport confirmation,
and B backtracking. Interactive map pan/zoom is outside this FELocation branch;
the original full-map default is retained. Frontend audio remains unsupported.

## Validation and check

Original IDA/asset read-only inspections and Rust syntax parsing only. No Cargo
compilation or runtime tests were run. This is not a claim
of pixel-perfect or runtime-verified parity. See movie evidence for remaining
screen-edge-offset and loaded-movie timing uncertainties.

Run `build-release.bat`, then `play-quiet.bat`. Press Start, select Map with A,
select Locations and a category, select a destination, then select Teleport.
Use D-pad or left stick for rows and B to back out. Check nearby and distant
destinations, return to the map, and confirm the arrival terrain is solid.
