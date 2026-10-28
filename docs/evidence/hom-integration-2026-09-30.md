# Hall of Meat HUD integration

Source implemented; compilation and runtime testing pending. Three parallel
agents traced native controller, completed physics data, and authored
movie independently. Root reviewed native UpdateHoM827c73b0 against the bridge
and retained controller, and integrates shared assets/rendering.

## Projection and draw order

Live IDA gScreenTemplate82fdb358, 16-byte entries:
- index12 /82fdb418: PhoneListHUD, hud2/hudphonelist.swf, transition0, type5
- index85 /82fdb8a8: TrickDisplay, hud2/trickdisplay2.swf, transition0, type5
- index115 /82fdba88: HOMScoringHUD, hud2/homscoring.swf, transition0, type0

PDB FEScreenType: type0=eFEScreenType_2D; type5=3D_HUD_Focus_Left.
Render2D82631a98 draws layer5 with CAMERAIDX_APT_3D, then switches to
CAMERAIDX_APT_SCREENSCAMERA and draws layer0. Therefore the HoM movie uses
its original flat1280x720 stage after the existing phone/trick HUD. It does
not inherit the focus-left tilt, jolt, scene fisheye, or scene brightness.
Native template registration82628098 confirms table use. Existing aspect-fit
presentation policy retained; full non-16:9 output parity not measured.

## Integration

Asset3 original homscoring apt/const/geo/rx2; FNV65f4830a. No replacement
artwork/fonts. Shared localization, glyph metrics, GPU textures and APT
timelines reused. Original animation ticks at16ms; score facts consumed per
completed physics frame, so uncapped rendering cannot duplicate awards.
Normal trick movie retains its original wipeout close path; no speculative
whole-HUD visibility override. kIsHoMHUDEnabled82f6b2d0 defaults1, retained
by selected session. HoM effect enable flag is still obeyed.

## Limits

Free-skate has no career challenge completion source, so completion count0 is
passed (challenge commands remain translated). Frontend sounds are retained
commands pending the existing absent UI audio backend. Asset/IDA inspection
and source review only; no build or runtime tests run for this change.
See hom-controller-2026-09-30.md, hom-physics-hud-ida-2026-09-30.txt,
and hom-movie-2026-09-30.md for their source contracts and limits.

Final separate read-only review by hom_controller found no API/field/call-site mismatches in shared integration. Movie agent confirmed zero direct bitmap placements; only shapes/text/sprites are placed. Compilation and visual verification remain pending.
