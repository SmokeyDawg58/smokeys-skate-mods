# Hall of Meat HUD controller — source ready, uncompiled

Confidence: high. Live IDA queries against the original executable/PDB on
2026-09-30 establish the following production contract. No compilation or tests
were run. Runtime appearance remains unverified.

## Original code and ownership

- `Sk8::HUD::HOMScoringHUD::HOMScoringHUD` `826b5860` starts hidden with zero
  completed challenges; it selects the HOMScoringHUD frontend screen.
- `HOMScoringHUD::Show` `826be050` compares visibility before emitting screen
  state 1 (show) or 2 (hide). Repeated show calls do not restart animation.
- `HOMScoringHUD::Update` `826be118` invokes its virtual Refresh when data is
  dirty, then independently tests completed-challenge count zero crossings:
  zero to nonzero emits 999; nonzero to zero emits 1000. Other changes do nothing.
- Original vtable `820ace60` has its Refresh slot at +8 pointing to `826be0b8`,
  symbolically merged with `OnlineObservingHUD::Refresh`. That implementation
  only acts while visible: emits state 3, then clears the data's dirty flag.
  Thus hidden data stays dirty. This was checked through actual vtable words,
  not inferred from the misleading folded function name.
- `HOMScoringHUDData::Reset` `826b1c38`, called by constructor `826b3cd8`, clears
  all scalar fields, all four ten-element collision arrays, and dirty=false.

PDB data layout is 244 bytes: base IHUDData at 0; score12, multiplier16,
rotation20, rotationScore24, speedScore28, dropScore32, airTimeScore36,
durationScore40, speed44, drop48, airTime52, duration56, newCar60, newPed64,
newDMO68, newSkater72, newBodyTweak76 (bool), carScore80[10], pedScore120[10],
dmoScore160[10], skaterScore200[10], bodyTweakScore240. Native score/mult are
signed int fields; the scoring producer writes unsigned score bits.

## Physics producer ordering

The existing `scoring-frontend-ida-2026-09-30.txt` native PhysOutModule_Hud
update records the order StartWipeout, UpdateHoM, UpdateWipeouts, then
ReactivateScoring. The new retained controller preserves this ordering.

- `StartWipeout` `827c7858` clears score only when nonzero, marks dirty, and
  shows according to `kIsHoMHUDEnabled && mHoM_IsEffectEnabled`.
  Native initializer `82f6b2d0` sets the HUD-enable debug variable to 1;
  the current runtime uses that default without a debug-variable service.
- `UpdateHoM` `827c73b0` requires bail-in-progress or bail-completed and the
  same enable conjunction. It copies total score; speed/drop/time/duration/
  rotation scores and metrics; newly collided cars/peds/DMOs; body-tweak flag
  and score. Every eligible update marks dirty even when values stay equal.
  Air-time is max-air-drop duration frames times native f32 `0.016666668`.
  Drop/time point scores come from accumulated air drop while metric values
  come from max air drop. Multiplier is not assigned. New-skater count and its
  first score are explicitly zeroed. Unwritten collision array tails persist.
- Collision setters `827c4008` / `827c40b0` / `827c4158` require index<10,
  write index%10, and mark dirty. Physics source entity arrays allow eight hits.
- `ReactivateScoring` `827c7230` hides, clears the four speed/drop/air/duration
  scores and corresponding metrics, and marks dirty. It does not reset score,
  rotation, multiplier, collision arrays, or body-tweak fields.
- `UpdateWipeouts` `827c47b8` updates medical-bill and jacked-overlay backend
  state; it does not directly hide this HUD.

StartWipeout also hides the minimap and remembers its prior visibility;
ReactivateScoring restores that visibility. Neither path hides TrickDisplayHUD.
No normal trick HUD suppression should be inferred merely from HOM visibility.

## Authored movie getter contract

`HOM_GetScore` `826ba608` and `HOM_GetMultiplier` `826ba648` return signed APT
integers. `HOM_GetBonusScores` `826ba688` truncates five float scores to integers
in speed, drop, air-time, duration, rotation order. `HOM_GetBonusData`
`826ba760` returns five floats in that same order. `HOM_GetBonusCollisionData`
`826ba7f8` returns car count, DMO count, body-tweak bool, pedestrian count,
skater count. `HOM_GetBonusCollisionScore` `826ba890` returns corresponding
score arrays with a scalar body-tweak score in slot2. The zero-count arrays
are empty (the native temporary blank string is not included in their length).
The native skater array getter reads the pedestrian score array; the live
producer always forces skater count zero, so that quirk is unobservable here.

`HOM_GetBonusMeasureArray` `826bfca0` reads results-screen StatsBackEnd data,
not the live scoring HUD data; it is not a substitute for the authored active
HUD scripts' numeric formatting.

## Implementation and limits

`ui/hom_scoring.rs` owns typed retained data, controller state, physics input
and frontend frame contract. Caller supplies the completed challenge count;
this module does not invent completed challenge records or score calculations.
The physics bridge and authored movie are separate peer-owned modules.
No executable/database/assets were copied into the repository.
