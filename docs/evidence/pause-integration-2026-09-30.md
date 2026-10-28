# Start pause menu integration

Source ready; compilation and controller testing pending. Three parallel
agents handled native controller, original APT movies, and pause/input runtime.
Root integrated assets, rendering, and frontend-only presentation and reviewed
source contracts. No Cargo build/check/test or game launch was run.

## Source and shared rendering

Original gScreenTemplate82fdb358 index11 at82fdb408 is StartHub,
starthub/starthubmenu.swf, transition0/type2. PDB type2 is3D_Focus_Left.
SetVertexMatrix826095e8 routes both2 and5 through FocusLeft826082b0, so
existing projection is reused. Native Render2D82631a98 order5->2->0 is retained,
including menu outro with restored gameplay HUD. No guessed scene dimmer:
StartHub gFEStateInfo.mDimmer=0 and mShowHUD=HideExceptOnline.

Assets4..8 contain original StartHub and four control imports; all admitted
by exact original FNV fingerprints. Original texture arenas/font atlases and
localization are shared. button_menu contains no RX2 and is not loaded as one.
Import resolution uses known exports and preserves the resolved asset owner.
Movie agent audited actions, InitActions, no positive masks or direct bitmap
placements. TextFormat overrides use native font/ascent/scale/advances, with
shared renderer support instead of mutated asset definitions.

Root independently queried cFont GetMaxCharHeight82849a80: raw load mAscent
at+0x164, multiplied by baseScale.Y at+0x80 via VMX128 before returning.
TextExtent82849ba0 accumulates glyph.advance times scale. Movie agent corrected
button baseline to match this and width to authored bounds, not texture pixels.

## Behavior

Start opens; D-pad or left-stick up/down navigates with original retained pad
repeat; Start or B closes. Original single-player list selected to include
Edit Skater; this is presentation only, not career activation.
Reference build includes Debug; Replay is disabled because no replay recorder
is present. All option destinations (including Quit) intentionally inactive.

Existing input owner polls once and drains SIM/PRES. Frontend state3 is pushed
through existing pause manager, returning before terrain/physics/scoring/camera
updates. UI animates over retained completed world. Closing input is consumed,
and paused time is not replayed as simulation debt. Native OnEnter normal-rate
message reused. Existing HUD hidden while open, restored on immediate native
unpause, with authored menu outro continuing. Frontend sounds remain commands
only because original UI audio playback is not connected yet.

## Review and remaining validation

Native agent independently reviewed runtime gate/cache/timer handling; movie
agent reviewed shared import/script/text hooks. Rustfmt parse-only inspection
of changed source succeeded, with no file rewrites. These are source checks,
not compilation or runtime verification. Check: build-release.bat, play-quiet.bat,
press Start, navigate, close/resume; also check opening during movement/bail.
No claim of measured pixel parity or complete frontend state-stack coverage.

Details: pause-native-2026-09-30.md, pause-movie-2026-09-30.md,
pause-runtime-2026-09-30.md, pause-root-ida-2026-09-30.txt.
